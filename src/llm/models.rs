//! Model catalog, base scores and the routing planner.
//!
//! Every model has a *base score* on three axes (reasoning, speed, cost —
//! 0..10, higher is better) plus a context window. A task is described by a
//! [`TaskProfile`]; the planner weights the axes by the task's complexity,
//! applies hard gates (context fit, provider availability, operator overrides)
//! and the governor's budget pressure, and returns an ordered attempt list.
//! The first attempt is the best fit; the rest are fallbacks across providers.
//!
//! Providers:
//!   * `claude`  — Claude Code CLI
//!   * `codex`   — ChatGPT Codex CLI (GPT-5.5, GPT-5.6 SOL/LUNA/TERRA, GPT-6 ASTRAS)
//!   * `gemini`  — Gemini CLI
//!   * `api`     — any OpenAI-compatible HTTP API from `.env` (Kimi, Qwen, DeepSeek…)
//!   * `hermes` / `opencode` / `custom` — generic CLIs (stdin prompt → stdout)

use super::Complexity;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Serialize)]
pub struct ModelSpec {
    pub id: String,
    pub provider: String,
    pub family: String,
    /// 0..10 — depth of reasoning / synthesis quality.
    pub reasoning: f32,
    /// 0..10 — latency (higher = faster).
    pub speed: f32,
    /// 0..10 — cost efficiency (higher = cheaper).
    pub cost: f32,
    /// Context window in thousands of tokens.
    pub context_k: u32,
    /// Rough USD per 1k tokens (input+output blended) for the governor.
    pub usd_per_1k: f32,
    pub strengths: Vec<String>,
}

fn m(id: &str, provider: &str, family: &str, reasoning: f32, speed: f32, cost: f32, context_k: u32, usd: f32, strengths: &[&str]) -> ModelSpec {
    ModelSpec { id: id.into(), provider: provider.into(), family: family.into(), reasoning, speed, cost, context_k, usd_per_1k: usd, strengths: strengths.iter().map(|s| s.to_string()).collect() }
}

/// Built-in catalog. Operators can extend/override with `CORTEX_MODELS_JSON`
/// (a JSON array of ModelSpec) — e.g. to register a newer model the moment it
/// ships without rebuilding.
pub fn catalog() -> Vec<ModelSpec> {
    let mut v = vec![
        // ---- Anthropic (Claude Code CLI) ----
        m("claude-opus-5", "claude", "claude", 10.0, 5.0, 3.0, 1000, 0.045, &["deep-reasoning", "synthesis", "long-context", "intelligence"]),
        m("claude-sonnet-5", "claude", "claude", 8.6, 8.0, 6.0, 1000, 0.012, &["structured", "extraction", "correlation", "balanced"]),
        m("claude-opus-4-8", "claude", "claude", 9.4, 5.0, 3.0, 200, 0.045, &["deep-reasoning", "legacy"]),
        m("claude-haiku-4-5-20251001", "claude", "claude", 6.5, 9.5, 9.0, 200, 0.003, &["fast", "classification", "cheap"]),
        // ---- OpenAI (Codex CLI) ----
        m("gpt-6-astras", "codex", "gpt", 9.8, 6.0, 3.0, 1000, 0.040, &["deep-reasoning", "agentic", "code"]),
        m("gpt-5.6-terra", "codex", "gpt", 9.0, 6.0, 4.0, 400, 0.025, &["agentic", "code", "long-tasks"]),
        m("gpt-5.6-sol", "codex", "gpt", 8.5, 8.0, 6.0, 400, 0.010, &["balanced", "structured", "code"]),
        m("gpt-5.6-luna", "codex", "gpt", 7.4, 9.2, 8.5, 400, 0.003, &["fast", "cheap", "classification"]),
        m("gpt-5.5", "codex", "gpt", 8.0, 7.0, 6.0, 400, 0.010, &["balanced", "code", "legacy"]),
        // ---- Google (Gemini CLI) ----
        m("gemini-2.5-pro", "gemini", "gemini", 8.5, 6.0, 5.0, 1000, 0.012, &["multimodal", "long-context", "geoint"]),
        m("gemini-2.5-flash", "gemini", "gemini", 7.0, 9.0, 9.0, 1000, 0.002, &["fast", "cheap", "multimodal"]),
        // ---- OpenAI-compatible HTTP APIs (enabled by CORTEX_API_<NAME>_KEY in .env) ----
        m("kimi-k2", "api:kimi", "kimi", 8.4, 7.0, 8.0, 256, 0.003, &["agentic", "long-context", "cheap"]),
        m("qwen3-235b-a22b", "api:qwen", "qwen", 8.2, 7.0, 8.5, 256, 0.002, &["multilingual", "code", "cheap"]),
        m("deepseek-v3.2", "api:deepseek", "deepseek", 8.3, 7.5, 9.0, 128, 0.001, &["cheap", "code", "structured"]),
        m("deepseek-r1", "api:deepseek", "deepseek", 8.9, 4.5, 8.0, 128, 0.003, &["deep-reasoning", "cheap"]),
        // ---- Generic CLIs (hermes / opencode / custom) ----
        m("hermes", "hermes", "cli", 7.5, 6.0, 9.5, 128, 0.0, &["local", "agentic"]),
        m("opencode", "opencode", "cli", 7.5, 6.0, 9.5, 128, 0.0, &["local", "code"]),
        m("custom", "custom", "cli", 7.0, 6.0, 9.5, 64, 0.0, &["local"]),
    ];
    if let Ok(extra) = std::env::var("CORTEX_MODELS_JSON") {
        if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&extra) {
            for e in list {
                let id = e.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() { continue; }
                let f = |k: &str, d: f32| e.get(k).and_then(|v| v.as_f64()).map(|x| x as f32).unwrap_or(d);
                let spec = ModelSpec {
                    id: id.clone(),
                    provider: e.get("provider").and_then(|v| v.as_str()).unwrap_or("custom").to_string(),
                    family: e.get("family").and_then(|v| v.as_str()).unwrap_or("custom").to_string(),
                    reasoning: f("reasoning", 7.0), speed: f("speed", 6.0), cost: f("cost", 7.0),
                    context_k: e.get("context_k").and_then(|v| v.as_u64()).unwrap_or(128) as u32,
                    usd_per_1k: f("usd_per_1k", 0.005),
                    strengths: e.get("strengths").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default(),
                };
                v.retain(|x| x.id != id);
                v.push(spec);
            }
        }
    }
    v
}

/// What the router needs to know about a request to score models.
#[derive(Clone, Debug, Serialize)]
pub struct TaskProfile {
    pub complexity: Complexity,
    pub needs_json: bool,
    pub est_tokens: u32,
    pub wants_code: bool,
    pub latency_sensitive: bool,
    /// 0..1 — governor pressure (1 = budget exhausted → strongly prefer cheap).
    pub budget_pressure: f32,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Scored {
    pub model: String,
    pub provider: String,
    pub score: f32,
    pub reason: String,
}

// ---- provider availability (cached) ----

fn availability() -> &'static Mutex<HashMap<String, (bool, std::time::Instant)>> {
    static A: OnceLock<Mutex<HashMap<String, (bool, std::time::Instant)>>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(HashMap::new()))
}

fn bin_exists(bin: &str) -> bool {
    if bin.contains('/') {
        return std::path::Path::new(bin).exists();
    }
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| std::path::Path::new(d).join(bin).exists())
}

/// Is a provider usable right now? Cheap check (binary on PATH / env present),
/// cached for 60s. Health failures at call time still fall through to the next
/// attempt — this only avoids scoring things that cannot possibly run.
pub fn provider_available(provider: &str) -> bool {
    {
        let a = availability().lock().unwrap();
        if let Some((ok, at)) = a.get(provider) {
            if at.elapsed().as_secs() < 60 {
                return *ok;
            }
        }
    }
    let ok = match provider {
        "claude" => bin_exists(&std::env::var("CORTEX_CLAUDE_BIN").unwrap_or_else(|_| "claude".into())),
        "codex" => bin_exists(&std::env::var("CORTEX_CODEX_BIN").unwrap_or_else(|_| "codex".into())),
        "gemini" => bin_exists(&std::env::var("CORTEX_GEMINI_BIN").unwrap_or_else(|_| "gemini".into())),
        "custom" => std::env::var("CORTEX_LLM_CMD").map(|s| !s.trim().is_empty()).unwrap_or(false),
        "hermes" => std::env::var("CORTEX_HERMES_CMD").map(|s| !s.trim().is_empty()).unwrap_or(false),
        "opencode" => std::env::var("CORTEX_OPENCODE_CMD").map(|s| !s.trim().is_empty()).unwrap_or(false),
        p if p.starts_with("api:") => super::api::is_configured(&p[4..]),
        _ => false,
    };
    availability().lock().unwrap().insert(provider.to_string(), (ok, std::time::Instant::now()));
    ok
}

/// Per-tier axis weights: (reasoning, speed, cost).
fn weights(c: Complexity) -> (f32, f32, f32) {
    match c {
        Complexity::Simple => (0.25, 0.40, 0.35),
        Complexity::Standard => (0.50, 0.25, 0.25),
        Complexity::Complex => (0.75, 0.10, 0.15),
    }
}

/// Score one model for a task. Returns None when hard-gated out.
pub fn score(spec: &ModelSpec, p: &TaskProfile) -> Option<Scored> {
    if !provider_available(&spec.provider) {
        return None;
    }
    // Context gate: payload + headroom for the answer must fit.
    let need_k = (p.est_tokens as f32 * 1.3 / 1000.0).ceil() as u32 + 8;
    if need_k > spec.context_k {
        return None;
    }
    let (wr, ws, wc) = weights(p.complexity);
    // Budget pressure shifts weight from reasoning to cost.
    let shift = 0.35 * p.budget_pressure;
    let wr2 = (wr - shift).max(0.15);
    let wc2 = wc + shift;
    let mut s = spec.reasoning * wr2 + spec.speed * ws + spec.cost * wc2;
    let mut why: Vec<String> = vec![format!("base r{:.1}/s{:.1}/c{:.1}", spec.reasoning, spec.speed, spec.cost)];
    let has = |k: &str| spec.strengths.iter().any(|x| x == k);
    if p.needs_json && has("structured") { s += 0.4; why.push("structured+".into()); }
    if p.wants_code && has("code") { s += 0.5; why.push("code+".into()); }
    if p.latency_sensitive && has("fast") { s += 0.5; why.push("fast+".into()); }
    if p.complexity == Complexity::Complex && has("deep-reasoning") { s += 0.6; why.push("deep+".into()); }
    if p.complexity == Complexity::Complex && has("intelligence") { s += 0.3; }
    if p.est_tokens > 60_000 && has("long-context") { s += 0.5; why.push("long-context+".into()); }
    if has("legacy") { s -= 0.6; }
    if p.budget_pressure > 0.7 && spec.usd_per_1k > 0.02 { s -= 1.5; why.push("budget-".into()); }
    Some(Scored { model: spec.id.clone(), provider: spec.provider.clone(), score: (s * 100.0).round() / 100.0, reason: why.join(" ") })
}

/// Ordered attempts for a task: best score first; providers deduplicated so
/// fallbacks jump across vendors (a Claude outage shouldn't retry Claude 3×).
/// Operator env overrides (`CORTEX_MODEL_COMPLEX/STANDARD/SIMPLE`) pin the
/// first attempt for that tier.
pub fn plan(p: &TaskProfile) -> Vec<Scored> {
    let cat = catalog();
    let mut scored: Vec<Scored> = cat.iter().filter_map(|s| score(s, p)).collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    // one best model per provider, in score order, then the remaining as deep fallbacks
    let mut seen = std::collections::HashSet::new();
    let mut primary: Vec<Scored> = Vec::new();
    let mut rest: Vec<Scored> = Vec::new();
    for s in scored {
        if seen.insert(s.provider.clone()) { primary.push(s); } else { rest.push(s); }
    }
    primary.extend(rest.into_iter().take(2));
    // Operator pin.
    let pin = match p.complexity {
        Complexity::Complex => std::env::var("CORTEX_MODEL_COMPLEX").ok(),
        Complexity::Standard => std::env::var("CORTEX_MODEL_STANDARD").ok(),
        Complexity::Simple => std::env::var("CORTEX_MODEL_SIMPLE").ok(),
    };
    if let Some(pin) = pin.filter(|s| !s.trim().is_empty()) {
        if let Some(spec) = cat.iter().find(|m| m.id == pin) {
            if provider_available(&spec.provider) {
                primary.retain(|s| s.model != pin);
                primary.insert(0, Scored { model: pin.clone(), provider: spec.provider.clone(), score: 999.0, reason: "operator override".into() });
            }
        } else {
            // unknown id: assume the pinned provider by prefix
            let prov = if pin.starts_with("claude") { "claude" } else if pin.starts_with("gpt") || pin.starts_with("o") { "codex" } else if pin.starts_with("gemini") { "gemini" } else { "custom" };
            if provider_available(prov) {
                primary.insert(0, Scored { model: pin.clone(), provider: prov.into(), score: 999.0, reason: "operator override (uncatalogued)".into() });
            }
        }
    }
    primary
}

/// Routing table for the settings UI: catalog + the winner per tier.
pub fn describe() -> serde_json::Value {
    let cat = catalog();
    let tiers = [("simple", Complexity::Simple), ("standard", Complexity::Standard), ("complex", Complexity::Complex)];
    let mut routes = serde_json::Map::new();
    for (name, c) in tiers {
        let p = TaskProfile { complexity: c, needs_json: true, est_tokens: 6000, wants_code: false, latency_sensitive: c == Complexity::Simple, budget_pressure: super::governor::pressure(), label: name.into() };
        routes.insert(name.to_string(), serde_json::to_value(plan(&p)).unwrap_or_default());
    }
    let models: Vec<serde_json::Value> = cat.iter().map(|s| {
        let mut v = serde_json::to_value(s).unwrap_or_default();
        v["available"] = serde_json::Value::Bool(provider_available(&s.provider));
        let base = s.reasoning * 0.5 + s.speed * 0.25 + s.cost * 0.25;
        v["base_score"] = serde_json::json!((base * 10.0).round() / 10.0);
        v
    }).collect();
    serde_json::json!({
        "models": models,
        "routes": routes,
        "governor": super::governor::snapshot(),
        "overrides": {
            "complex": std::env::var("CORTEX_MODEL_COMPLEX").ok(),
            "standard": std::env::var("CORTEX_MODEL_STANDARD").ok(),
            "simple": std::env::var("CORTEX_MODEL_SIMPLE").ok(),
        },
        "providers": providers_json()
    })
}

fn providers_json() -> Vec<serde_json::Value> {
    ["claude", "codex", "gemini", "api:kimi", "api:qwen", "api:deepseek", "api:openrouter", "api:ollama", "hermes", "opencode", "custom"]
        .iter()
        .map(|p| serde_json::json!({"id": p, "available": provider_available(p)}))
        .collect()
}

/// Cost/latency spec lookup for the governor.
pub fn spec(id: &str) -> Option<ModelSpec> {
    catalog().into_iter().find(|m| m.id == id)
}
