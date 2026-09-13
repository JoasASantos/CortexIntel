//! Token / cost governor. Tracks estimated tokens and USD per job and globally,
//! exposes a 0..1 *pressure* the router uses to drift towards cheaper models
//! as a job burns through its budget, and can veto a call outright when the
//! hard ceiling is hit. Budgets are self-tuned from the task: a run over a
//! large dataset gets more headroom than an interactive question.
//!
//! Env: `CORTEX_TOKEN_BUDGET` (per job, default 400k), `CORTEX_USD_BUDGET`
//! (per job, default 3.0), `CORTEX_TOKEN_BUDGET_GLOBAL` (session, default 5M).

use crate::bus;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Usage {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub usd: f64,
    pub calls: u32,
    pub cache_hits: u32,
    pub budget_tokens: u64,
    pub budget_usd: f64,
}

struct State {
    jobs: HashMap<String, Usage>,
    global: Usage,
}

fn st() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(State { jobs: HashMap::new(), global: Usage { budget_tokens: env_u64("CORTEX_TOKEN_BUDGET_GLOBAL", 5_000_000), budget_usd: env_f64("CORTEX_USD_BUDGET_GLOBAL", 40.0), ..Default::default() } }))
}

fn env_u64(k: &str, d: u64) -> u64 { std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d) }
fn env_f64(k: &str, d: f64) -> f64 { std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d) }

/// Rough token estimate (chars/4 with a floor for CJK/emoji-heavy text).
pub fn estimate_tokens(text: &str) -> u32 {
    let chars = text.chars().count() as u32;
    let non_ascii = text.chars().filter(|c| !c.is_ascii()).count() as u32;
    (chars / 4 + non_ascii / 2).max(1)
}

/// Start tracking a job with a budget sized to the expected workload.
/// `scale` ≥ 1 multiplies the default budget (runs over big datasets pass the
/// number of records / 500, interactive asks pass 1).
pub fn begin(job: &str, scale: f64) {
    let scale = scale.clamp(0.25, 8.0);
    let u = Usage {
        budget_tokens: (env_u64("CORTEX_TOKEN_BUDGET", 400_000) as f64 * scale) as u64,
        budget_usd: env_f64("CORTEX_USD_BUDGET", 3.0) * scale,
        ..Default::default()
    };
    bus::emit("governor.begin", format!("budget {} tokens / ${:.2}", u.budget_tokens, u.budget_usd));
    st().lock().unwrap().jobs.insert(job.to_string(), u);
}

pub fn end(job: &str) -> Option<Usage> {
    let u = st().lock().unwrap().jobs.remove(job);
    if let Some(u) = &u {
        bus::emit("governor.end", format!("used {} in / {} out tokens · ${:.4} · {} calls · {} cache hits", u.tokens_in, u.tokens_out, u.usd, u.calls, u.cache_hits));
    }
    u
}

fn current_job() -> Option<String> { bus::current_job() }

/// Record a completed call.
pub fn record(model: &str, tokens_in: u32, tokens_out: u32) {
    let per_1k = if model == "offline" || model == "mock" { 0.0 } else { super::models::spec(model).map(|m| m.usd_per_1k as f64).unwrap_or(0.005) };
    let usd = per_1k * ((tokens_in + tokens_out) as f64 / 1000.0);
    let mut s = st().lock().unwrap();
    s.global.tokens_in += tokens_in as u64; s.global.tokens_out += tokens_out as u64; s.global.usd += usd; s.global.calls += 1;
    if let Some(j) = current_job() {
        if let Some(u) = s.jobs.get_mut(&j) {
            u.tokens_in += tokens_in as u64; u.tokens_out += tokens_out as u64; u.usd += usd; u.calls += 1;
            let pct = (u.tokens_in + u.tokens_out) as f64 / u.budget_tokens.max(1) as f64;
            drop(s);
            bus::emit("governor.usage", format!("{model}: +{tokens_in}/{tokens_out} tokens · ${usd:.4} · job at {:.0}% of budget", pct * 100.0));
            return;
        }
    }
    drop(s);
    bus::emit("governor.usage", format!("{model}: +{tokens_in}/{tokens_out} tokens · ${usd:.4}"));
}

pub fn record_cache_hit() {
    let mut s = st().lock().unwrap();
    s.global.cache_hits += 1;
    if let Some(j) = current_job() { if let Some(u) = s.jobs.get_mut(&j) { u.cache_hits += 1; } }
}

/// 0..1 — how much of the budget the current job has consumed (global if no job).
pub fn pressure() -> f32 {
    let s = st().lock().unwrap();
    let u = current_job().and_then(|j| s.jobs.get(&j).cloned()).unwrap_or_else(|| s.global.clone());
    let t = (u.tokens_in + u.tokens_out) as f64 / u.budget_tokens.max(1) as f64;
    let d = u.usd / u.budget_usd.max(0.01);
    t.max(d).clamp(0.0, 1.0) as f32
}

/// Veto: may we spend `est_tokens` more? Hard ceiling at 120% of the budget so a
/// last synthesis step still completes; beyond that the call is refused.
pub fn allow(est_tokens: u32) -> Result<(), String> {
    let s = st().lock().unwrap();
    let u = current_job().and_then(|j| s.jobs.get(&j).cloned()).unwrap_or_else(|| s.global.clone());
    let after = u.tokens_in + u.tokens_out + est_tokens as u64;
    if after as f64 > u.budget_tokens as f64 * 1.2 {
        return Err(format!("token budget exhausted ({} of {} tokens used)", u.tokens_in + u.tokens_out, u.budget_tokens));
    }
    if u.usd > u.budget_usd * 1.2 {
        return Err(format!("cost budget exhausted (${:.2} of ${:.2})", u.usd, u.budget_usd));
    }
    Ok(())
}

pub fn snapshot() -> serde_json::Value {
    let s = st().lock().unwrap();
    serde_json::json!({ "global": s.global, "pressure": pressure_of(&s.global), "jobs": s.jobs.len() })
}

fn pressure_of(u: &Usage) -> f32 {
    let t = (u.tokens_in + u.tokens_out) as f64 / u.budget_tokens.max(1) as f64;
    t.clamp(0.0, 1.0) as f32
}
