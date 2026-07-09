//! Neural layer (embeddings) — consistent with CortexIntel's philosophy of
//! driving the operator's own tools. An embedder is any CLI set via
//! `CORTEX_EMBED_CMD` that reads text on stdin and writes a JSON array of floats
//! on stdout (e.g. a wrapper around a local sentence-transformers / Ollama embed
//! model). Similarity (cosine) and nearest-neighbour search are done here in Rust.
//!
//! This powers semantic similarity, fuzzy dedup and — with an image/face embedder
//! — the victim-identification vector match. All matches are decision-support
//! requiring human confirmation and are gated behind the governance block.

use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Is a neural embedder configured?
pub fn is_configured() -> bool {
    std::env::var("CORTEX_EMBED_CMD").ok().filter(|s| !s.trim().is_empty()).is_some()
}

/// Embed a piece of text into a vector via the configured CLI embedder.
pub fn embed(text: &str) -> Result<Vec<f32>> {
    let cmd = std::env::var("CORTEX_EMBED_CMD").ok().filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("CORTEX_EMBED_CMD not set — no embedder configured"))?;
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let (bin, args) = parts.split_first().ok_or_else(|| anyhow!("empty CORTEX_EMBED_CMD"))?;
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn embedder `{bin}`"))?;
    if let Some(mut sin) = child.stdin.take() {
        let _ = sin.write_all(text.as_bytes());
    }
    let out = child.wait_with_output().context("embedder process failed")?;
    if !out.status.success() {
        return Err(anyhow!("embedder exited: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let v: Vec<f32> = serde_json::from_slice(&out.stdout)
        .context("embedder must output a JSON array of numbers")?;
    if v.is_empty() {
        return Err(anyhow!("embedder returned an empty vector"));
    }
    Ok(v)
}

/// Cosine similarity in [-1, 1]; 0 if either vector is degenerate or mismatched.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Nearest neighbours of `query` among `pool` by cosine, above `min_sim`.
/// Returns (index, similarity) sorted by similarity desc — a brute-force ANN
/// that's fine for the served slice; a real ANN index comes later for scale.
pub fn nearest(query: &[f32], pool: &[Vec<f32>], min_sim: f32, top_k: usize) -> Vec<(usize, f32)> {
    let mut scored: Vec<(usize, f32)> = pool.iter().enumerate()
        .map(|(i, v)| (i, cosine(query, v)))
        .filter(|(_, s)| *s >= min_sim)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored.truncate(top_k);
    scored
}
