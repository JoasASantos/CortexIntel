//! Robustness signals for messy real-world data: fuzzy near-duplicate correlation
//! and temporal correlation. Both are deterministic and offline, and add edges the
//! exact-match correlator can't see.
//!
//!  * Fuzzy: entities of the same kind whose labels are highly similar (typos,
//!    accents, name-order, abbreviations) are linked `similar_to` — the seed of
//!    identity resolution on dirty data.
//!  * Temporal: entities active in the same narrow time window are linked
//!    `co_active_with` — a time analogue of the shared-hub correlator.

use crate::ontology::{EntityKind, KnowledgeGraph, Relationship};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FuzzyStats {
    pub similar_links: usize,
    pub temporal_links: usize,
}

pub fn apply(graph: &mut KnowledgeGraph) -> FuzzyStats {
    FuzzyStats {
        similar_links: fuzzy_labels(graph),
        temporal_links: temporal(graph),
    }
}

/// Normalize a label for comparison: lowercase, strip accents, drop non-alnum,
/// collapse spaces. "João A. Silva" and "joao a silva" → "joao a silva".
fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let c = deaccent(ch).to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if c.is_whitespace() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.trim().to_string()
}
fn deaccent(c: char) -> char {
    match c {
        'á'|'à'|'â'|'ã'|'ä'|'Á'|'À'|'Â'|'Ã'|'Ä' => 'a',
        'é'|'è'|'ê'|'ë'|'É'|'È'|'Ê'|'Ë' => 'e',
        'í'|'ì'|'î'|'ï'|'Í'|'Ì'|'Î'|'Ï' => 'i',
        'ó'|'ò'|'ô'|'õ'|'ö'|'Ó'|'Ò'|'Ô'|'Õ'|'Ö' => 'o',
        'ú'|'ù'|'û'|'ü'|'Ú'|'Ù'|'Û'|'Ü' => 'u',
        'ç'|'Ç' => 'c', 'ñ'|'Ñ' => 'n',
        _ => c,
    }
}

/// Token-set Jaccard similarity in [0,1] — order-insensitive, good for names.
fn jaccard(a: &str, b: &str) -> f32 {
    let sa: std::collections::HashSet<&str> = a.split(' ').filter(|t| !t.is_empty()).collect();
    let sb: std::collections::HashSet<&str> = b.split(' ').filter(|t| !t.is_empty()).collect();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count() as f32;
    let uni = sa.union(&sb).count() as f32;
    inter / uni
}

/// Normalized edit-distance similarity in [0,1] — catches typos within a token.
fn edit_sim(a: &str, b: &str) -> f32 {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        return 0.0;
    }
    if (n as i32 - m as i32).abs() as usize > n.max(m) / 2 {
        return 0.0; // lengths too different — cheap reject
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let dist = prev[m] as f32;
    1.0 - dist / n.max(m) as f32
}

/// Link entities of the same kind whose labels are highly similar. Only kinds
/// where name variance is meaningful (people, orgs, accounts).
fn fuzzy_labels(graph: &mut KnowledgeGraph) -> usize {
    let kinds = [EntityKind::Person, EntityKind::Victim, EntityKind::Suspect, EntityKind::Organization, EntityKind::Account];
    let items: Vec<(String, EntityKind, String)> = graph.entities.iter()
        .filter(|(_, e)| kinds.contains(&e.kind))
        .map(|(id, e)| (id.clone(), e.kind, norm(&e.label)))
        .filter(|(_, _, n)| n.len() >= 4)
        .collect();
    if items.len() > 2000 {
        return 0; // O(n²) guard for very large graphs (an index comes later)
    }
    let mut edges = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            if items[i].1 != items[j].1 || items[i].2 == items[j].2 {
                continue;
            }
            let j_sim = jaccard(&items[i].2, &items[j].2);
            let e_sim = edit_sim(&items[i].2, &items[j].2);
            let sim = j_sim.max(e_sim);
            if sim >= 0.8 {
                let mut r = Relationship::new(items[i].0.clone(), "similar_to", items[j].0.clone(), sim.min(0.95));
                r.source_reference = Some(format!("fuzzy:{:.2}", sim));
                edges.push(r);
            }
        }
    }
    add_all(graph, edges)
}

/// Best-effort epoch-ish minute from a timestamp string (ISO or "YYYY-MM-DD HH").
fn time_bucket(ts: &str, window_min: i64) -> Option<i64> {
    let t = ts.trim();
    // Extract YYYY, MM, DD, HH, MM digits positionally where possible.
    let digits: Vec<i64> = {
        let mut nums = Vec::new();
        let mut cur = String::new();
        for c in t.chars() {
            if c.is_ascii_digit() { cur.push(c); }
            else if !cur.is_empty() { nums.push(cur.parse().unwrap_or(0)); cur.clear(); }
        }
        if !cur.is_empty() { nums.push(cur.parse().unwrap_or(0)); }
        nums
    };
    if digits.len() < 3 {
        return None;
    }
    let (y, mo, d) = (digits[0], digits.get(1).copied().unwrap_or(1), digits.get(2).copied().unwrap_or(1));
    let hh = digits.get(3).copied().unwrap_or(0);
    let mi = digits.get(4).copied().unwrap_or(0);
    if !(1900..=2100).contains(&y) {
        return None;
    }
    // Coarse minutes-since-2000 (approximate; only relative buckets matter).
    let minutes = (((y - 2000) * 365 + mo * 31 + d) * 24 + hh) * 60 + mi;
    Some(minutes / window_min.max(1))
}

/// Link entities sharing a narrow activity window (co-temporal). Window in minutes
/// via CORTEX_TEMPORAL_WINDOW_MIN (default 60).
fn temporal(graph: &mut KnowledgeGraph) -> usize {
    let window: i64 = std::env::var("CORTEX_TEMPORAL_WINDOW_MIN").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut by_bucket: HashMap<i64, Vec<String>> = HashMap::new();
    for (id, e) in &graph.entities {
        let ts = e.attributes.get("timestamp")
            .or_else(|| e.attributes.get("created_at"))
            .or_else(|| e.attributes.get("received_at"))
            .or_else(|| e.attributes.get("observed_at"))
            .or_else(|| e.attributes.get("date"));
        if let Some(ts) = ts {
            if let Some(b) = time_bucket(ts, window) {
                by_bucket.entry(b).or_default().push(id.clone());
            }
        }
    }
    // Skip pairs already directly connected (same-record entities share a
    // timestamp) — the signal is co-activity between OTHERWISE-unrelated entities.
    let connected: std::collections::HashSet<(String, String)> = graph.relationships.iter()
        .map(|r| if r.source_id <= r.target_id { (r.source_id.clone(), r.target_id.clone()) } else { (r.target_id.clone(), r.source_id.clone()) })
        .collect();
    let is_conn = |a: &str, b: &str| {
        let k = if a <= b { (a.to_string(), b.to_string()) } else { (b.to_string(), a.to_string()) };
        connected.contains(&k)
    };
    let mut edges = Vec::new();
    for (_b, ids) in &by_bucket {
        let mut u: Vec<&String> = ids.iter().collect();
        u.sort(); u.dedup();
        if u.len() < 2 || u.len() > 10 {
            continue; // a giant time bucket is noise, not signal
        }
        for i in 0..u.len() {
            for j in (i + 1)..u.len() {
                if is_conn(u[i], u[j]) {
                    continue;
                }
                let mut r = Relationship::new(u[i].clone(), "co_active_with", u[j].clone(), 0.45);
                r.source_reference = Some("temporal:same-window".into());
                edges.push(r);
            }
        }
    }
    add_all(graph, edges)
}

fn add_all(graph: &mut KnowledgeGraph, edges: Vec<Relationship>) -> usize {
    let mut n = 0;
    for e in edges {
        let before = graph.relationship_count();
        graph.add_relationship(e);
        if graph.relationship_count() > before { n += 1; }
    }
    n
}
