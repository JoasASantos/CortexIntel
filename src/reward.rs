//! Reward / feedback engine: the loop that makes the decision engine learn from
//! the analyst. When a human confirms or rejects an assessment, action or entity,
//! that verdict is recorded as a reward signal keyed by a dimension (entity kind,
//! tag, or action type). Future risk scoring reads a bounded adjustment per key,
//! so what the operator repeatedly confirms is prioritized and what they reject is
//! down-weighted. Transparent (plain counts, bounded nudge) — not a black box.
//!
//! Persisted to `<data-dir>/reward.json`. Adjustments are clamped to ±0.15 so
//! feedback nudges the deterministic score, never overrides it.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_ADJUST: f32 = 0.15;

#[derive(Debug, Default, Serialize, Deserialize)]
struct RewardStore {
    /// key ("kind:account", "tag:known-file-hash", "action:verify") -> net signal.
    #[serde(default)]
    net: HashMap<String, f32>,
    /// key -> number of feedback events (for confidence in the adjustment).
    #[serde(default)]
    count: HashMap<String, u32>,
}

fn path() -> std::path::PathBuf {
    crate::store::base_dir().join("reward.json")
}

fn load() -> RewardStore {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(store: &RewardStore) {
    if let Ok(s) = serde_json::to_string_pretty(store) {
        let _ = std::fs::write(path(), s);
    }
}

/// Record a feedback event. `signal` is typically +1 (confirm) or -1 (reject);
/// `weight` scales it (default 1.0). Keys are free-form dimensions.
pub fn record(key: &str, signal: f32, weight: f32) {
    if key.trim().is_empty() {
        return;
    }
    let mut s = load();
    *s.net.entry(key.to_string()).or_insert(0.0) += signal * weight;
    *s.count.entry(key.to_string()).or_insert(0) += 1;
    save(&s);
}

/// Bounded per-key adjustment in [-MAX_ADJUST, +MAX_ADJUST]. The nudge grows with
/// agreement but saturates, and is damped when there are few events (uncertain).
pub fn adjustments() -> HashMap<String, f32> {
    let s = load();
    let mut out = HashMap::new();
    for (k, net) in &s.net {
        let n = *s.count.get(k).unwrap_or(&1) as f32;
        // saturating tanh-like curve; confidence factor n/(n+2) damps sparse keys.
        let conf = n / (n + 2.0);
        let adj = (net / (net.abs() + 3.0)) * MAX_ADJUST * conf;
        if adj.abs() > 1e-4 {
            out.insert(k.clone(), adj);
        }
    }
    out
}

/// Total adjustment for an entity from its kind + tags (used by risk scoring).
pub fn entity_adjustment(adj: &HashMap<String, f32>, kind: &str, tags: &[String]) -> f32 {
    if adj.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    if let Some(v) = adj.get(&format!("kind:{kind}")) {
        total += v;
    }
    for t in tags {
        if let Some(v) = adj.get(&format!("tag:{t}")) {
            total += v;
        }
    }
    total.clamp(-MAX_ADJUST, MAX_ADJUST)
}
