//! Prompt cache (exact) + semantic cache (near-duplicate) for LLM calls.
//!
//! Layer 0 — exact: FNV hash of (system, prompt, schema) → stored reply.
//! Layer 1 — semantic: when `CORTEX_EMBED_CMD` is configured the prompt is
//! embedded and cosine ≥ 0.965 against stored prompts of the same agent label
//! is a hit; without an embedder a lexical Jaccard ≥ 0.92 fingerprint match is
//! used instead. Both layers live under `<data>/cache/llm/` with a TTL
//! (`CORTEX_LLM_CACHE_TTL_H`, default 72h). Set `CORTEX_LLM_CACHE=0` to disable.

use super::prep;
use crate::bus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    key: u64,
    label: String,
    model: String,
    provider: String,
    created: u64,
    #[serde(default)]
    fp: Vec<u64>,
    #[serde(default)]
    emb: Vec<f32>,
    file: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Index {
    entries: Vec<Entry>,
}

fn dir() -> PathBuf {
    crate::store::base_dir().join("cache").join("llm")
}

fn idx() -> &'static Mutex<Index> {
    static I: OnceLock<Mutex<Index>> = OnceLock::new();
    I.get_or_init(|| {
        let p = dir().join("index.json");
        let i: Index = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Mutex::new(i)
    })
}

fn enabled() -> bool {
    std::env::var("CORTEX_LLM_CACHE").map(|v| v != "0").unwrap_or(true)
}

fn ttl_secs() -> u64 {
    std::env::var("CORTEX_LLM_CACHE_TTL_H").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(72) * 3600
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn key_of(system: &str, prompt: &str, schema: &Option<serde_json::Value>) -> u64 {
    let s = format!("{}\u{1}{}\u{1}{}", system, prompt, schema.as_ref().map(|v| v.to_string()).unwrap_or_default());
    prep::fnv(&s)
}

fn persist(i: &Index) {
    let _ = crate::store::ensure_dir(&dir());
    let _ = std::fs::write(dir().join("index.json"), serde_json::to_vec(i).unwrap_or_default());
}

/// Look up a cached reply. Exact first, then semantic (same agent label only —
/// a risk assessment must never be served from an extraction cache).
pub fn lookup(label: &str, system: &str, prompt: &str, schema: &Option<serde_json::Value>) -> Option<super::LlmResponse> {
    if !enabled() { return None; }
    let key = key_of(system, prompt, schema);
    let ttl = ttl_secs();
    let t = now();
    let mut i = idx().lock().unwrap();
    // expire
    let before = i.entries.len();
    i.entries.retain(|e| t.saturating_sub(e.created) < ttl);
    if i.entries.len() != before { persist(&i); }
    if let Some(e) = i.entries.iter().find(|e| e.key == key) {
        if let Some(r) = read_entry(e) {
            bus::emit("cache.hit", format!("prompt cache · {} · {}", e.label, e.model));
            super::governor::record_cache_hit();
            return Some(r);
        }
    }
    // semantic layer
    let fp = prep::fingerprint(prompt);
    let emb = if crate::embed::is_configured() { crate::embed::embed(&prompt.chars().take(8000).collect::<String>()).ok() } else { None };
    let mut best: Option<(f32, &Entry)> = None;
    for e in i.entries.iter().filter(|e| e.label == label) {
        let sim = if let (Some(q), false) = (&emb, e.emb.is_empty()) { crate::embed::cosine(q, &e.emb) } else { prep::jaccard(&fp, &e.fp) };
        if best.map(|(b, _)| sim > b).unwrap_or(true) { best = Some((sim, e)); }
    }
    if let Some((sim, e)) = best {
        let thr = if emb.is_some() && !e.emb.is_empty() { 0.965 } else { 0.92 };
        if sim >= thr {
            if let Some(r) = read_entry(e) {
                bus::emit("cache.hit", format!("semantic cache · sim {:.3} · {} · {}", sim, e.label, e.model));
                super::governor::record_cache_hit();
                return Some(r);
            }
        }
    }
    None
}

fn read_entry(e: &Entry) -> Option<super::LlmResponse> {
    let b = std::fs::read(dir().join(&e.file)).ok()?;
    serde_json::from_slice(&b).ok()
}

pub fn store(label: &str, system: &str, prompt: &str, schema: &Option<serde_json::Value>, resp: &super::LlmResponse) {
    if !enabled() { return; }
    let key = key_of(system, prompt, schema);
    let file = format!("{:016x}.json", key);
    let _ = crate::store::ensure_dir(&dir());
    if std::fs::write(dir().join(&file), serde_json::to_vec(resp).unwrap_or_default()).is_err() { return; }
    let emb = if crate::embed::is_configured() { crate::embed::embed(&prompt.chars().take(8000).collect::<String>()).unwrap_or_default() } else { Vec::new() };
    let e = Entry { key, label: label.into(), model: resp.model.clone(), provider: resp.provider.clone(), created: now(), fp: prep::fingerprint(prompt), emb, file };
    let mut i = idx().lock().unwrap();
    i.entries.retain(|x| x.key != key);
    i.entries.push(e);
    let n = i.entries.len();
    if n > 2000 { i.entries.drain(0..n - 2000); }
    persist(&i);
}

pub fn stats() -> serde_json::Value {
    let i = idx().lock().unwrap();
    serde_json::json!({ "entries": i.entries.len(), "dir": dir().to_string_lossy(), "enabled": enabled(), "ttl_h": ttl_secs() / 3600, "semantic": if crate::embed::is_configured() { "embeddings" } else { "lexical" } })
}

pub fn clear() -> usize {
    let mut i = idx().lock().unwrap();
    let n = i.entries.len();
    for e in &i.entries { let _ = std::fs::remove_file(dir().join(&e.file)); }
    i.entries.clear();
    persist(&i);
    n
}
