//! Request preparation and response distillation — the "treat before it
//! reaches Cortex" layer.
//!
//! * **Observation compressor** (`compact`): whitespace collapse, duplicate-line
//!   removal, long-line truncation and head/tail windowing so a prompt never
//!   carries more than the model needs.
//! * **Language directive**: every request states the answer language (pt-BR
//!   by default for Portuguese operators) so UI text and AI output agree.
//! * **Distill** (`distill`): strips fences, boilerplate and prose around the
//!   JSON, trims, and — when a schema was requested — keeps only the parsed JSON
//!   value, so only the essential payload flows into the engine.

use std::collections::HashSet;

/// Compress free text to at most `max_chars`, keeping structure.
pub fn compact(text: &str, max_chars: usize) -> String {
    let mut seen: HashSet<u64> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    let mut blank_run = 0usize;
    for raw in text.lines() {
        let line = raw.trim_end();
        let norm = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if norm.is_empty() {
            blank_run += 1;
            if blank_run <= 1 { out.push(String::new()); }
            continue;
        }
        blank_run = 0;
        // dedupe exact repeated lines (common in exported logs / CSV echoes),
        // but keep short structural lines like "{" / "-" / headings.
        if norm.len() > 24 {
            let h = fnv(&norm);
            if !seen.insert(h) { continue; }
        }
        let mut l = if line.starts_with(' ') || line.starts_with('\t') {
            // preserve indentation depth (JSON / lists) but collapse inner runs
            let indent = line.len() - line.trim_start().len();
            format!("{}{}", " ".repeat(indent.min(8)), norm)
        } else { norm };
        if l.chars().count() > 600 {
            l = l.chars().take(600).collect::<String>() + " …";
        }
        out.push(l);
    }
    let mut joined = out.join("\n");
    if joined.chars().count() > max_chars {
        // head + tail windowing: keep the instructions (head) and the most
        // recent data (tail); drop the middle with a marker.
        let chars: Vec<char> = joined.chars().collect();
        let head = (max_chars as f64 * 0.62) as usize;
        let tail = max_chars - head;
        let dropped = chars.len() - max_chars;
        joined = format!("{}\n\n[… {} chars compressed …]\n\n{}", chars[..head].iter().collect::<String>(), dropped, chars[chars.len() - tail..].iter().collect::<String>());
    }
    joined
}

/// Language line prepended to the system prompt.
pub fn language_directive(lang: &str) -> &'static str {
    match lang {
        "pt" => "IDIOMA: Responda SEMPRE em português do Brasil (pt-BR) — todos os textos, resumos, justificativas, rótulos e recomendações. Mantenha identificadores técnicos (IPs, domínios, hashes, e-mails, nomes de campos JSON) exatamente como estão. Seja direto: só o essencial, sem preâmbulos.",
        "es" => "IDIOMA: Responde SIEMPRE en español — todos los textos, resúmenes, justificaciones y recomendaciones. Mantén los identificadores técnicos tal cual. Sé directo: solo lo esencial.",
        _ => "LANGUAGE: Always answer in English. Keep technical identifiers verbatim. Be direct: only what is essential, no preamble.",
    }
}

/// Post-process a raw model reply. Returns (clean_text, parsed_json_if_any).
pub fn distill(raw: &str, expects_json: bool) -> (String, Option<serde_json::Value>) {
    let mut t = raw.trim().to_string();
    // strip common assistant boilerplate lines
    let boiler = ["as an ai", "here is the json", "here's the json", "aqui está o json", "segue o json", "sure!", "certainly", "claro!"];
    t = t.lines().filter(|l| { let low = l.trim().to_lowercase(); !(low.len() < 60 && boiler.iter().any(|b| low.starts_with(b))) }).collect::<Vec<_>>().join("\n");
    if expects_json {
        if let Ok(v) = super::extract_json(&t) {
            let v = prune_json(v, 0);
            let s = serde_json::to_string(&v).unwrap_or_default();
            return (s, Some(v));
        }
    }
    // prose: collapse whitespace, cap size
    let mut s = compact(&t, 20_000);
    // remove stray code fences left over
    s = s.replace("```json", "").replace("```", "");
    (s.trim().to_string(), None)
}

/// Drop empty strings / arrays / nulls from a JSON tree so only meaningful data
/// reaches the engine and the UI (depth-limited to avoid pathological trees).
pub fn prune_json(v: serde_json::Value, depth: usize) -> serde_json::Value {
    use serde_json::Value::*;
    if depth > 12 { return v; }
    match v {
        Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                let p = prune_json(val, depth + 1);
                let empty = match &p { Null => true, String(s) => s.trim().is_empty(), Array(a) => a.is_empty(), Object(o) => o.is_empty(), _ => false };
                if !empty { out.insert(k, p); }
            }
            Object(out)
        }
        Array(a) => Array(a.into_iter().map(|x| prune_json(x, depth + 1)).filter(|x| !matches!(x, Null)).collect()),
        String(s) => String(s.trim().to_string()),
        other => other,
    }
}

pub fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Cheap lexical fingerprint for the semantic cache fallback: the set of
/// lower-cased word tokens ≥ 3 chars (hashed) — Jaccard over these is a decent
/// paraphrase detector when no embedder is configured.
pub fn fingerprint(text: &str) -> Vec<u64> {
    let mut v: Vec<u64> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '.' && c != '@' && c != '-')
        .filter(|w| w.len() >= 3)
        .map(fnv)
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

pub fn jaccard(a: &[u64], b: &[u64]) -> f32 {
    if a.is_empty() || b.is_empty() { return 0.0; }
    let (mut i, mut j, mut inter) = (0usize, 0usize, 0usize);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => { inter += 1; i += 1; j += 1; }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    inter as f32 / (a.len() + b.len() - inter) as f32
}
