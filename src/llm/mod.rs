//! The LLM decision layer. CortexIntel does not embed model weights or call any
//! HTTP API directly — it drives the operator's *already-authenticated* CLIs:
//!
//!   * `claude`  — Claude Code (subscription) in headless print mode with
//!                 `--dangerously-skip-permissions`.
//!   * `codex`   — ChatGPT Codex `codex exec`.
//!
//! Every agent talks to a [`LlmRouter`] which picks a backend and enforces a
//! JSON-in / JSON-out contract so results can be parsed deterministically.

pub mod api;
pub mod cache;
mod claude;
mod codex;
mod gemini;
mod generic;
pub mod governor;
mod mock;
pub mod models;
pub mod prep;

pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use gemini::GeminiProvider;
pub use generic::GenericProvider;
pub use mock::MockProvider;
pub use api::ApiProvider;

use crate::bus;
use crate::config::ProviderChoice;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// macOS/Linux GUI apps launched from Finder/Dock do NOT inherit the shell PATH,
/// so `claude`/`codex` (in ~/.local/bin, /opt/homebrew/bin, …) are invisible and
/// spawning fails. Prepend the common CLI locations to PATH once at startup so
/// the embedded server can find them regardless of how the app was launched.
pub fn augment_path() {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join("bin"));
        dirs.push(home.join(".cargo/bin"));
    }
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/usr/bin", "/bin"] {
        dirs.push(std::path::PathBuf::from(d));
    }
    let cur = std::env::var("PATH").unwrap_or_default();
    let existing: std::collections::HashSet<&str> = cur.split(':').collect();
    let mut prefix: Vec<String> = Vec::new();
    for d in &dirs {
        let s = d.to_string_lossy().to_string();
        if d.exists() && !existing.contains(s.as_str()) {
            prefix.push(s);
        }
    }
    if !prefix.is_empty() {
        let joined = if cur.is_empty() { prefix.join(":") } else { format!("{}:{}", prefix.join(":"), cur) };
        std::env::set_var("PATH", joined);
    }
}

/// A single completion request.
/// How demanding a task is — drives model routing in `Auto` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// Answer language ("en" | "pt" | "es"); None = router default.
    pub lang: Option<String>,
    /// Routing hints.
    pub wants_code: bool,
    pub latency_sensitive: bool,
    /// Skip the prompt/semantic cache (e.g. re-runs the operator forced).
    pub no_cache: bool,
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
            lang: None,
            wants_code: false,
            latency_sensitive: false,
            no_cache: false,
        }
    }

    pub fn lang(mut self, l: impl Into<String>) -> Self {
        self.lang = Some(l.into());
        self
    }

    pub fn code(mut self) -> Self {
        self.wants_code = true;
        self
    }

    pub fn fast(mut self) -> Self {
        self.latency_sensitive = true;
        self
    }

    pub fn fresh(mut self) -> Self {
        self.no_cache = true;
        self
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
    gemini: GeminiProvider,
    generic: GenericProvider,
    hermes: GenericProvider,
    opencode: GenericProvider,
    mock: MockProvider,
    verbose: bool,
    lang: String,
}

/// Per-request tokens after preparation (for governor accounting).
fn est(req: &LlmRequest) -> u32 {
    governor::estimate_tokens(&req.system) + governor::estimate_tokens(&req.prompt)
}

impl LlmRouter {
    pub fn new(
        choice: ProviderChoice,
        claude_model: Option<String>,
        codex_model: Option<String>,
        verbose: bool,
    ) -> Self {
        api::load_dotenv();
        LlmRouter {
            choice,
            claude: ClaudeProvider::new(claude_model),
            codex: CodexProvider::new(codex_model),
            gemini: GeminiProvider::new(None),
            generic: GenericProvider::new(),
            hermes: GenericProvider::named("hermes", "CORTEX_HERMES_CMD"),
            opencode: GenericProvider::named("opencode", "CORTEX_OPENCODE_CMD"),
            mock: MockProvider::default(),
            verbose,
            lang: std::env::var("CORTEX_LANG").unwrap_or_else(|_| "pt".into()),
        }
    }

    /// Force the mock backend (offline mode).
    pub fn offline(verbose: bool) -> Self {
        let mut r = Self::new(ProviderChoice::Mock, None, None, verbose);
        r.choice = ProviderChoice::Mock;
        r
    }

    /// Set the answer language for every request routed through this router.
    pub fn with_lang(mut self, lang: &str) -> Self {
        self.lang = match lang { "pt" | "es" | "en" => lang.into(), _ => "pt".into() };
        self
    }

    pub fn choice(&self) -> ProviderChoice {
        self.choice
    }

    /// Prepare a request: compress the observation payload, prepend the
    /// language directive, and mark the effective language.
    fn prepare(&self, req: &LlmRequest) -> LlmRequest {
        let mut r = req.clone();
        let lang = r.lang.clone().unwrap_or_else(|| self.lang.clone());
        r.lang = Some(lang.clone());
        let before = r.prompt.chars().count();
        // Budget-aware compression: tighter when the governor is under pressure.
        let p = governor::pressure();
        let max = if p > 0.8 { 24_000 } else if p > 0.5 { 48_000 } else { 120_000 };
        r.prompt = prep::compact(&r.prompt, max);
        let after = r.prompt.chars().count();
        if before != after {
            bus::emit("prep.compress", format!("{}: prompt {} → {} chars", r.agent_label, before, after));
        }
        r.system = format!("{}\n\n{}", prep::language_directive(&lang), r.system);
        r
    }

    /// Distill a raw provider reply down to the essential payload.
    fn finish(&self, req: &LlmRequest, mut resp: LlmResponse, tokens_in: u32) -> LlmResponse {
        let raw_len = resp.text.chars().count();
        let (clean, parsed) = prep::distill(&resp.text, req.json_schema.is_some());
        if req.json_schema.is_some() && parsed.is_none() {
            bus::emit("prep.distill", format!("{}: reply is not JSON — kept as text ({} chars)", req.agent_label, raw_len));
        } else if clean.chars().count() != raw_len {
            bus::emit("prep.distill", format!("{}: reply {} → {} chars", req.agent_label, raw_len, clean.chars().count()));
        }
        resp.text = clean;
        governor::record(&resp.model, tokens_in, governor::estimate_tokens(&resp.text));
        resp
    }

    /// Run a request according to the routing policy. `Auto` scores every
    /// catalogued model for the task (base score × task weights × budget
    /// pressure) and walks the ranked list until one succeeds; explicit
    /// provider choices still get the tier's best model of that provider.
    pub fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let req = self.prepare(req);
        if self.verbose {
            eprintln!("  · llm[{}] ← agent={} complexity={:?}", self.choice, req.agent_label, req.complexity);
        }
        if self.choice == ProviderChoice::Mock {
            let r = self.mock.complete(&req)?;
            return Ok(self.finish(&req, r, est(&req)));
        }
        // cache
        if !req.no_cache {
            if let Some(hit) = cache::lookup(&req.agent_label, &req.system, &req.prompt, &req.json_schema) {
                return Ok(hit);
            }
        }
        let tokens_in = est(&req);
        if let Err(e) = governor::allow(tokens_in) {
            bus::emit("governor.veto", format!("{}: {e}", req.agent_label));
            return Err(anyhow!("governor: {e}"));
        }
        let profile = models::TaskProfile {
            complexity: req.complexity,
            needs_json: req.json_schema.is_some(),
            est_tokens: tokens_in,
            wants_code: req.wants_code,
            latency_sensitive: req.latency_sensitive,
            budget_pressure: governor::pressure(),
            label: req.agent_label.clone(),
        };
        let mut plan = models::plan(&profile);
        // Explicit provider choice: keep only that provider's attempts (best first).
        let only: Option<&str> = match self.choice {
            ProviderChoice::Claude => Some("claude"),
            ProviderChoice::Codex => Some("codex"),
            ProviderChoice::Gemini => Some("gemini"),
            ProviderChoice::Custom => Some("custom"),
            _ => None,
        };
        if let Some(p) = only {
            let filtered: Vec<models::Scored> = plan.iter().filter(|a| a.provider == p).cloned().collect();
            plan = if filtered.is_empty() {
                // provider not detected on PATH — still try its default model
                vec![models::Scored { model: default_model_for(p, req.complexity), provider: p.into(), score: 0.0, reason: "forced".into() }]
            } else { filtered };
        }
        if plan.is_empty() {
            return Err(anyhow!("no LLM provider available — install claude/codex/gemini CLI or set CORTEX_API_<NAME>_KEY / CORTEX_LLM_CMD"));
        }
        bus::emit_data("llm.route", format!("{} [{:?}, ~{} tok, pressure {:.0}%] → {}", req.agent_label, req.complexity, tokens_in, profile.budget_pressure * 100.0,
            plan.iter().take(3).map(|a| format!("{}:{} ({})", a.provider, a.model, a.score)).collect::<Vec<_>>().join(" › ")),
            Some(serde_json::json!({"plan": plan.iter().map(|a| serde_json::json!({"provider":a.provider,"model":a.model,"score":a.score,"why":a.reason})).collect::<Vec<_>>()})));
        let mut errors = Vec::new();
        for attempt in plan {
            let mut r = req.clone();
            // An explicit per-request model wins over the routed one.
            if r.model.is_none() && !["custom", "hermes", "opencode"].contains(&attempt.provider.as_str()) {
                r.model = Some(attempt.model.clone());
            }
            let started = std::time::Instant::now();
            bus::emit("llm.call", format!("{} → {}:{}", req.agent_label, attempt.provider, attempt.model));
            let res = with_retry(|| self.dispatch(&attempt.provider, &r));
            match res {
                Ok(v) => {
                    bus::emit("llm.ok", format!("{} ← {}:{} in {:.1}s", req.agent_label, v.provider, v.model, started.elapsed().as_secs_f32()));
                    let out = self.finish(&req, v, tokens_in);
                    if !req.no_cache { cache::store(&req.agent_label, &req.system, &req.prompt, &req.json_schema, &out); }
                    return Ok(out);
                }
                Err(e) => {
                    bus::emit("llm.fallback", format!("{}:{} failed ({}) → next", attempt.provider, attempt.model, short(&e.to_string())));
                    errors.push(format!("{}={e}", attempt.provider));
                }
            }
        }
        Err(anyhow!("all routed providers failed: {}", errors.join("; ")))
    }

    fn dispatch(&self, provider: &str, r: &LlmRequest) -> Result<LlmResponse> {
        match provider {
            "claude" => self.claude.complete(r),
            "codex" => self.codex.complete(r),
            "gemini" => self.gemini.complete(r),
            "custom" => self.generic.complete(r),
            "hermes" => self.hermes.complete(r),
            "opencode" => self.opencode.complete(r),
            p if p.starts_with("api:") => ApiProvider::new(&p[4..]).complete(r),
            other => Err(anyhow!("unknown provider {other}")),
        }
    }

    /// Health of every backend, for `cortex doctor`.
    pub fn health_report(&self) -> Vec<(String, Result<String>)> {
        let mut v = vec![
            ("claude".into(), self.claude.health()),
            ("codex".into(), self.codex.health()),
            ("gemini".into(), self.gemini.health()),
            ("custom".into(), self.generic.health()),
            ("hermes".into(), self.hermes.health()),
            ("opencode".into(), self.opencode.health()),
            ("mock".into(), self.mock.health()),
        ];
        for name in ["kimi", "qwen", "deepseek", "openrouter", "ollama"] {
            let p = ApiProvider::new(name);
            v.push((format!("api:{name}"), p.health()));
        }
        v
    }
}

fn default_model_for(provider: &str, c: Complexity) -> String {
    match (provider, c) {
        ("claude", Complexity::Complex) => "claude-opus-5".into(),
        ("claude", _) => "claude-sonnet-5".into(),
        ("codex", Complexity::Complex) => "gpt-6-astras".into(),
        ("codex", Complexity::Simple) => "gpt-5.6-luna".into(),
        ("codex", _) => "gpt-5.6-sol".into(),
        ("gemini", _) => "gemini-2.5-pro".into(),
        _ => "custom".into(),
    }
}

fn short(s: &str) -> String {
    let s = s.lines().next().unwrap_or(s);
    s.chars().take(140).collect()
}

/// Retry a completion on transient failure (backoff between tries). Count is
/// `CORTEX_LLM_RETRIES` (default 1 extra try). Makes the harness more robust to
/// flaky CLIs / rate limits without masking a hard failure.
fn with_retry<F: Fn() -> Result<LlmResponse>>(f: F) -> Result<LlmResponse> {
    let retries: u32 = std::env::var("CORTEX_LLM_RETRIES").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut last: Option<anyhow::Error> = None;
    for attempt in 0..=retries {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last = Some(e);
                if attempt < retries {
                    std::thread::sleep(std::time::Duration::from_millis(400 * (attempt as u64 + 1)));
                }
            }
        }
    }
    Err(last.unwrap_or_else(|| anyhow!("request failed")))
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
