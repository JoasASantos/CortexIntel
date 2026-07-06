//! The LLM decision layer. CortexIntel does not embed model weights or call any
//! HTTP API directly — it drives the operator's *already-authenticated* CLIs:
//!
//!   * `claude`  — Claude Code (subscription) in headless print mode with
//!                 `--dangerously-skip-permissions`.
//!   * `codex`   — ChatGPT Codex `codex exec`.
//!
//! Every agent talks to a [`LlmRouter`] which picks a backend and enforces a
//! JSON-in / JSON-out contract so results can be parsed deterministically.

mod claude;
mod codex;
mod mock;

pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use mock::MockProvider;

use crate::config::ProviderChoice;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// A single completion request.
/// How demanding a task is — drives model routing in `Auto` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    /// Cheap, mechanical (classification, governance summary) → Codex.
    Simple,
    /// Structured work (extraction, correlation) → Claude Sonnet.
    Standard,
    /// Deep reasoning / synthesis / high data density → Claude Opus.
    Complex,
}

#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// System / role instructions (agent persona + guardrails).
    pub system: String,
    /// The user prompt (task + data payload).
    pub prompt: String,
    /// If set, the provider is asked to emit JSON matching this schema and the
    /// router validates the result parses as JSON.
    pub json_schema: Option<serde_json::Value>,
    /// Optional model override for this call (wins over routing).
    pub model: Option<String>,
    /// Label used only for logging/telemetry.
    pub agent_label: String,
    /// Task complexity tier — used by the Auto router to pick a model.
    pub complexity: Complexity,
}

impl LlmRequest {
    pub fn new(system: impl Into<String>, prompt: impl Into<String>) -> Self {
        LlmRequest {
            system: system.into(),
            prompt: prompt.into(),
            json_schema: None,
            model: None,
            agent_label: "agent".into(),
            complexity: Complexity::Standard,
        }
    }

    pub fn json(mut self, schema: serde_json::Value) -> Self {
        self.json_schema = Some(schema);
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.agent_label = l.into();
        self
    }

    pub fn complexity(mut self, c: Complexity) -> Self {
        self.complexity = c;
        self
    }

    /// Escalate to Complex when the payload is large (high data density benefits
    /// from Claude's bigger context and stronger synthesis).
    pub fn density_aware(mut self) -> Self {
        if self.prompt.len() > 12_000 {
            self.complexity = Complexity::Complex;
        }
        self
    }
}

/// One routing attempt: which backend + which model.
#[derive(Debug, Clone)]
struct Attempt {
    provider: &'static str, // "claude" | "codex"
    model: String,
}

fn model_complex() -> String { std::env::var("CORTEX_MODEL_COMPLEX").unwrap_or_else(|_| "claude-opus-4-8".into()) }
fn model_standard() -> String { std::env::var("CORTEX_MODEL_STANDARD").unwrap_or_else(|_| "claude-sonnet-5".into()) }
fn model_simple() -> String { std::env::var("CORTEX_MODEL_SIMPLE").unwrap_or_else(|_| "gpt-5.5".into()) }

/// Ordered provider/model attempts for a complexity tier (first is preferred,
/// rest are fallbacks). Critical/high-density → Claude Opus then Sonnet; simple
/// → Codex first. Every plan can still fall back across providers.
fn route_plan(c: Complexity) -> Vec<Attempt> {
    match c {
        Complexity::Complex => vec![
            Attempt { provider: "claude", model: model_complex() },
            Attempt { provider: "claude", model: model_standard() },
            Attempt { provider: "codex", model: model_simple() },
        ],
        Complexity::Standard => vec![
            Attempt { provider: "claude", model: model_standard() },
            Attempt { provider: "codex", model: model_simple() },
        ],
        Complexity::Simple => vec![
            Attempt { provider: "codex", model: model_simple() },
            Attempt { provider: "claude", model: model_standard() },
        ],
    }
}

/// A completion result plus provenance the audit log needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub text: String,
    pub provider: String,
    pub model: String,
}

impl LlmResponse {
    /// Best-effort extraction of a JSON value from a model response that may be
    /// wrapped in prose or ```json fences.
    pub fn as_json(&self) -> Result<serde_json::Value> {
        extract_json(&self.text)
    }
}

/// The contract every backend implements.
pub trait LlmProvider: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse>;
    /// Cheap liveness check (binary present + authenticated).
    fn health(&self) -> Result<String>;
}

/// Routes requests to one or more backends with a fallback policy.
pub struct LlmRouter {
    choice: ProviderChoice,
    claude: ClaudeProvider,
    codex: CodexProvider,
    mock: MockProvider,
    verbose: bool,
}

impl LlmRouter {
    pub fn new(
        choice: ProviderChoice,
        claude_model: Option<String>,
        codex_model: Option<String>,
        verbose: bool,
    ) -> Self {
        LlmRouter {
            choice,
            claude: ClaudeProvider::new(claude_model),
            codex: CodexProvider::new(codex_model),
            mock: MockProvider::default(),
            verbose,
        }
    }

    /// Force the mock backend (offline mode).
    pub fn offline(verbose: bool) -> Self {
        LlmRouter {
            choice: ProviderChoice::Mock,
            claude: ClaudeProvider::new(None),
            codex: CodexProvider::new(None),
            mock: MockProvider::default(),
            verbose,
        }
    }

    pub fn choice(&self) -> ProviderChoice {
        self.choice
    }

    /// Run a request according to the routing policy. In `Auto` mode the task's
    /// complexity picks the model chain: critical/high-density → Claude Opus 4.8
    /// (then Sonnet), simpler → Codex (gpt-5.5). Explicit provider choices still
    /// get a complexity-appropriate default model when none was set.
    pub fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        if self.verbose {
            eprintln!("  · llm[{}] ← agent={} complexity={:?}", self.choice, req.agent_label, req.complexity);
        }
        match self.choice {
            ProviderChoice::Mock => self.mock.complete(req),
            ProviderChoice::Claude => {
                let mut r = req.clone();
                if r.model.is_none() {
                    r.model = Some(if req.complexity == Complexity::Complex { model_complex() } else { model_standard() });
                }
                self.claude.complete(&r)
            }
            ProviderChoice::Codex => {
                let mut r = req.clone();
                if r.model.is_none() { r.model = Some(model_simple()); }
                self.codex.complete(&r)
            }
            ProviderChoice::Auto => self.route_auto(req),
        }
    }

    fn route_auto(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let plan = route_plan(req.complexity);
        let mut errors = Vec::new();
        for attempt in plan {
            let mut r = req.clone();
            // An explicit per-request model wins over the routed one.
            if r.model.is_none() {
                r.model = Some(attempt.model.clone());
            }
            if self.verbose {
                eprintln!("    → try {}::{}", attempt.provider, attempt.model);
            }
            let res = match attempt.provider {
                "claude" => self.claude.complete(&r),
                _ => self.codex.complete(&r),
            };
            match res {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if self.verbose {
                        eprintln!("      ✗ {} failed: {e}", attempt.provider);
                    }
                    errors.push(format!("{}={e}", attempt.provider));
                }
            }
        }
        Err(anyhow!("all routed providers failed: {}", errors.join("; ")))
    }

    /// Health of every backend, for `cortex doctor`.
    pub fn health_report(&self) -> Vec<(String, Result<String>)> {
        vec![
            ("claude".into(), self.claude.health()),
            ("codex".into(), self.codex.health()),
            ("mock".into(), self.mock.health()),
        ]
    }
}

/// Extract a JSON object/array from arbitrary model output.
pub fn extract_json(text: &str) -> Result<serde_json::Value> {
    let trimmed = text.trim();
    // Direct parse.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Ok(v);
    }
    // Fenced ```json ... ``` block.
    if let Some(inner) = extract_fenced(trimmed) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&inner) {
            return Ok(v);
        }
    }
    // First balanced { .. } or [ .. ] span.
    if let Some(span) = first_balanced_span(trimmed) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&span) {
            return Ok(v);
        }
    }
    Err(anyhow!("no parseable JSON found in model output"))
}

fn extract_fenced(text: &str) -> Option<String> {
    let start = text.find("```")?;
    let after = &text[start + 3..];
    // Skip an optional language tag on the same line.
    let body_start = after.find('\n').map(|i| i + 1).unwrap_or(0);
    let body = &after[body_start..];
    let end = body.find("```")?;
    Some(body[..end].trim().to_string())
}

fn first_balanced_span(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let open = text.find(['{', '['])?;
    let open_ch = bytes[open] as char;
    let close_ch = if open_ch == '{' { '}' } else { ']' };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, ch) in text[open..].char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            c if c == open_ch => depth += 1,
            c if c == close_ch => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[open..open + i + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}
