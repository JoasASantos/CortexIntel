//! Memory & retrieval layers.
//!
//! * **Project memory** — durable facts/notes/answers per project
//!   (`<data>/memory/<project>/facts.json`), written by the orchestrator after
//!   every intelligence step and by analysts (notes), recalled on demand.
//! * **Retrieval index** — every entity, relationship, comment, activity, brief
//!   and memory fact of a project becomes a document. Three retrievers:
//!     - `grep`   — substring / regex scan (exact selectors, hashes, IDs);
//!     - `bm25`   — lexical ranking (BM25 over word tokens, no deps);
//!     - `vector` — cosine over embeddings when `CORTEX_EMBED_CMD` is set
//!       (vectors are cached on disk by content hash).
//!   `hybrid` fuses them with reciprocal-rank fusion.
//! * **GraphRAG** — top entity hits are expanded one hop through the
//!   relationship graph so the model sees the *neighbourhood*, not isolated
//!   rows, and the result is packed into a size-bounded context block.
//! * **Context compression** — the packed context goes through the observation
//!   compressor with a char budget derived from the governor's pressure.

use crate::bus;
use crate::llm::prep;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Project memory (facts)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fact {
    pub id: String,
    /// fact | hypothesis | note | answer | decision
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created: u64,
    #[serde(default)]
    pub entity_ids: Vec<String>,
}

fn mem_dir(project: &str) -> PathBuf {
    crate::store::base_dir().join("memory").join(project.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect::<String>())
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn facts(project: &str) -> Vec<Fact> {
    crate::store::read_json_or_default::<Vec<Fact>>(&mem_dir(project).join("facts.json"))
}

pub fn remember(project: &str, kind: &str, text: &str, source: &str, entity_ids: &[String]) -> Option<Fact> {
    let text = text.trim();
    if text.is_empty() || project.is_empty() { return None; }
    let mut all = facts(project);
    let key = prep::fnv(&text.to_lowercase());
    if all.iter().any(|f| prep::fnv(&f.text.to_lowercase()) == key) { return None; }
    let f = Fact { id: format!("m-{:012x}", key), kind: kind.into(), text: text.chars().take(2000).collect(), source: source.into(), tags: vec![], created: now(), entity_ids: entity_ids.to_vec() };
    all.push(f.clone());
    if all.len() > 5000 { all.drain(0..all.len() - 5000); }
    let _ = crate::store::ensure_dir(&mem_dir(project));
    let _ = crate::store::write_json(&mem_dir(project).join("facts.json"), &all);
    invalidate(project);
    bus::emit("memory.write", format!("{kind}: {}", text.chars().take(90).collect::<String>()));
    Some(f)
}

pub fn forget(project: &str, id: &str) -> bool {
    let mut all = facts(project);
    let n = all.len();
    all.retain(|f| f.id != id);
    if all.len() != n {
        let _ = crate::store::write_json(&mem_dir(project).join("facts.json"), &all);
        invalidate(project);
        true
    } else { false }
}

// ---------------------------------------------------------------------------
// Retrieval index
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Doc {
    pub id: String,
    /// entity | relationship | comment | activity | brief | memory
    pub kind: String,
    pub title: String,
    pub text: String,
    #[serde(skip)]
    pub tokens: Vec<String>,
    #[serde(skip)]
    pub emb: Vec<f32>,
    #[serde(default)]
    pub entity_ids: Vec<String>,
}

struct Index {
    built_at: u64,
    version: u64,
    docs: Vec<Doc>,
    df: HashMap<String, u32>,
    avg_len: f32,
    /// entity id → (label, kind)
    entities: HashMap<String, (String, String)>,
    /// entity id → neighbours (id, rel_type, direction)
    adj: HashMap<String, Vec<(String, String, bool)>>,
}

fn indexes() -> &'static Mutex<HashMap<String, Index>> {
    static I: OnceLock<Mutex<HashMap<String, Index>>> = OnceLock::new();
    I.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn invalidate(project: &str) {
    indexes().lock().unwrap().remove(project);
}

pub fn tokenize(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in s.to_lowercase().split(|c: char| !c.is_alphanumeric() && c != '.' && c != '@' && c != '-' && c != '_' && c != ':') {
        let w = w.trim_matches(|c: char| c == '.' || c == '-' || c == ':' || c == '_');
        if w.len() < 2 { continue; }
        out.push(w.to_string());
        // compound selectors (example.onion, user@host, a-b) also index their parts
        if w.contains(['.', '@', '-', '_', ':']) {
            for part in w.split(['.', '@', '-', '_', ':']) {
                if part.len() >= 3 && !part.chars().all(|c| c.is_ascii_digit()) { out.push(part.to_string()); }
            }
        }
    }
    out
}

fn value_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a.iter().map(value_text).collect::<Vec<_>>().join(", "),
        serde_json::Value::Object(o) => o.iter().map(|(k, v)| format!("{k}: {}", value_text(v))).collect::<Vec<_>>().join("; "),
        other => other.to_string(),
    }
}

/// Build (or reuse) the index for a project. `graph` is the frontend's current
/// graph (nodes/edges) when available — it wins over the stored result because
/// it reflects analyst edits.
fn ensure_index(project: &str, graph: Option<&serde_json::Value>) {
    let proj = crate::projects::load(project).ok();
    let version = proj.as_ref().map(|p| p.updated_at).unwrap_or(0) ^ graph.map(|g| prep::fnv(&g.to_string())).unwrap_or(0);
    {
        let m = indexes().lock().unwrap();
        if let Some(i) = m.get(project) {
            if i.version == version && now() - i.built_at < 3600 { return; }
        }
    }
    let started = std::time::Instant::now();
    let mut docs: Vec<Doc> = Vec::new();
    let mut entities: HashMap<String, (String, String)> = HashMap::new();
    let mut adj: HashMap<String, Vec<(String, String, bool)>> = HashMap::new();
    // --- graph: nodes + edges
    let (nodes, edges): (Vec<serde_json::Value>, Vec<serde_json::Value>) = if let Some(g) = graph.filter(|g| g.get("nodes").and_then(|n| n.as_array()).map(|a| !a.is_empty()).unwrap_or(false)) {
        (g["nodes"].as_array().cloned().unwrap_or_default(), g.get("edges").and_then(|e| e.as_array()).cloned().unwrap_or_default())
    } else if let Some(r) = proj.as_ref().and_then(|p| p.last_result.clone()) {
        let mut ns = Vec::new();
        if let Some(o) = r.get("entities").and_then(|e| e.as_object()) { for arr in o.values() { if let Some(a) = arr.as_array() { ns.extend(a.iter().cloned()); } } }
        (ns, r.get("relationships").and_then(|e| e.as_array()).cloned().unwrap_or_default())
    } else { (vec![], vec![]) };
    for n in &nodes {
        let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let kind = n.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        let label = n.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if id.is_empty() { continue; }
        let risk = n.get("risk").and_then(|v| v.as_f64()).or_else(|| n.get("risk_score").and_then(|v| v.as_f64())).unwrap_or(0.0);
        let attrs = n.get("attributes").map(value_text).unwrap_or_default();
        let tags = n.get("tags").map(value_text).unwrap_or_default();
        let text = format!("{label} [{kind}] risk {risk:.2}. {attrs}. tags: {tags}");
        entities.insert(id.clone(), (label.clone(), kind.clone()));
        docs.push(Doc { id: id.clone(), kind: "entity".into(), title: label, text, tokens: vec![], emb: vec![], entity_ids: vec![id] });
    }
    for e in &edges {
        let s = e.get("source").or_else(|| e.get("source_id")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let t = e.get("target").or_else(|| e.get("target_id")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let ty = e.get("type").or_else(|| e.get("rel_type")).and_then(|v| v.as_str()).unwrap_or("related").to_string();
        if s.is_empty() || t.is_empty() { continue; }
        adj.entry(s.clone()).or_default().push((t.clone(), ty.clone(), true));
        adj.entry(t.clone()).or_default().push((s.clone(), ty.clone(), false));
        let sl = entities.get(&s).map(|x| x.0.clone()).unwrap_or(s.clone());
        let tl = entities.get(&t).map(|x| x.0.clone()).unwrap_or(t.clone());
        docs.push(Doc { id: format!("rel:{s}:{t}"), kind: "relationship".into(), title: format!("{sl} → {ty} → {tl}"), text: format!("{sl} {} {tl}", ty.replace('_', " ")), tokens: vec![], emb: vec![], entity_ids: vec![s, t] });
    }
    // --- project text: comments, activities, brief/assessment, memory facts
    if let Some(p) = &proj {
        for c in &p.comments {
            docs.push(Doc { id: format!("cmt:{}", c.id), kind: "comment".into(), title: format!("comment by {}", c.author), text: c.text.clone(), tokens: vec![], emb: vec![], entity_ids: vec![c.object_id.clone()] });
        }
        for a in p.activities.iter().rev().take(60) {
            docs.push(Doc { id: format!("act:{}", a.id), kind: "activity".into(), title: a.kind.clone(), text: a.summary.clone(), tokens: vec![], emb: vec![], entity_ids: vec![] });
        }
        if let Some(r) = &p.last_result {
            for key in ["investigation", "assessment", "ai_assessments", "next_best_actions"] {
                if let Some(v) = r.get(key) {
                    let text = prep::compact(&value_text(v), 6000);
                    if !text.trim().is_empty() {
                        docs.push(Doc { id: format!("brief:{key}"), kind: "brief".into(), title: key.replace('_', " "), text, tokens: vec![], emb: vec![], entity_ids: vec![] });
                    }
                }
            }
        }
    }
    for f in facts(project) {
        docs.push(Doc { id: f.id.clone(), kind: format!("memory:{}", f.kind), title: f.kind.clone(), text: f.text.clone(), tokens: vec![], emb: vec![], entity_ids: f.entity_ids.clone() });
    }
    // --- lexical stats
    let mut df: HashMap<String, u32> = HashMap::new();
    let mut total = 0usize;
    for d in docs.iter_mut() {
        d.tokens = tokenize(&format!("{} {}", d.title, d.text));
        total += d.tokens.len();
        let uniq: HashSet<&String> = d.tokens.iter().collect();
        for t in uniq { *df.entry(t.clone()).or_insert(0) += 1; }
    }
    let avg_len = if docs.is_empty() { 1.0 } else { total as f32 / docs.len() as f32 };
    // --- embeddings (optional, cached by content hash)
    if crate::embed::is_configured() {
        let cache_dir = crate::store::base_dir().join("cache").join("emb");
        let _ = crate::store::ensure_dir(&cache_dir);
        let mut embedded = 0;
        for d in docs.iter_mut().take(4000) {
            let h = prep::fnv(&d.text);
            let f = cache_dir.join(format!("{h:016x}.json"));
            if let Ok(b) = std::fs::read(&f) { if let Ok(v) = serde_json::from_slice::<Vec<f32>>(&b) { d.emb = v; continue; } }
            if let Ok(v) = crate::embed::embed(&d.text.chars().take(2000).collect::<String>()) {
                let _ = std::fs::write(&f, serde_json::to_vec(&v).unwrap_or_default());
                d.emb = v; embedded += 1;
            }
            if embedded > 500 { break; } // bound first-build cost; the rest embed lazily on later builds
        }
    }
    bus::emit("memory.index", format!("{} docs ({} entities, {} edges) indexed in {:.0}ms", docs.len(), entities.len(), edges.len(), started.elapsed().as_secs_f32() * 1000.0));
    indexes().lock().unwrap().insert(project.to_string(), Index { built_at: now(), version, docs, df, avg_len, entities, adj });
}

#[derive(Clone, Debug, Serialize)]
pub struct Hit {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub score: f32,
    pub entity_ids: Vec<String>,
    pub via: String,
}

fn bm25(idx: &Index, q: &[String], k: usize) -> Vec<(usize, f32)> {
    let n = idx.docs.len() as f32;
    let (k1, b) = (1.4f32, 0.75f32);
    let mut scored: Vec<(usize, f32)> = Vec::new();
    for (i, d) in idx.docs.iter().enumerate() {
        let mut s = 0.0f32;
        let dl = d.tokens.len() as f32;
        for term in q {
            let tf = d.tokens.iter().filter(|t| *t == term).count() as f32;
            if tf == 0.0 {
                // prefix match for partial identifiers ("exam" → example.onion)
                let pf = d.tokens.iter().filter(|t| term.len() >= 4 && t.starts_with(term.as_str())).count() as f32;
                if pf == 0.0 { continue; }
                let df = *idx.df.get(term).unwrap_or(&1) as f32;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                s += 0.5 * idf * (pf * (k1 + 1.0)) / (pf + k1 * (1.0 - b + b * dl / idx.avg_len));
                continue;
            }
            let df = *idx.df.get(term).unwrap_or(&1) as f32;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            s += idf * (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / idx.avg_len));
        }
        if s > 0.0 { scored.push((i, s)); }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored
}

fn grep_scan(idx: &Index, pattern: &str, k: usize) -> Vec<(usize, f32)> {
    let low = pattern.to_lowercase();
    let mut out = Vec::new();
    for (i, d) in idx.docs.iter().enumerate() {
        let hay = format!("{} {}", d.title, d.text).to_lowercase();
        if hay.contains(&low) {
            // exact title match ranks first
            let s = if d.title.to_lowercase() == low { 3.0 } else if d.title.to_lowercase().contains(&low) { 2.0 } else { 1.0 };
            out.push((i, s));
        }
    }
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(k);
    out
}

fn vector(idx: &Index, query: &str, k: usize) -> Vec<(usize, f32)> {
    if !crate::embed::is_configured() { return vec![]; }
    let Ok(q) = crate::embed::embed(query) else { return vec![] };
    let mut out: Vec<(usize, f32)> = idx.docs.iter().enumerate().filter(|(_, d)| !d.emb.is_empty()).map(|(i, d)| (i, crate::embed::cosine(&q, &d.emb))).filter(|(_, s)| *s > 0.25).collect();
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(k);
    out
}

/// Hybrid retrieval: grep (exact selectors) + BM25 + vectors fused with RRF.
pub fn search(project: &str, query: &str, k: usize, graph: Option<&serde_json::Value>, mode: &str) -> Vec<Hit> {
    ensure_index(project, graph);
    let m = indexes().lock().unwrap();
    let Some(idx) = m.get(project) else { return vec![] };
    let q = tokenize(query);
    let mut fused: HashMap<usize, (f32, String)> = HashMap::new();
    let mut add = |list: Vec<(usize, f32)>, via: &str, w: f32| {
        for (rank, (i, _)) in list.iter().enumerate() {
            let e = fused.entry(*i).or_insert((0.0, String::new()));
            e.0 += w / (60.0 + rank as f32);
            if !e.1.contains(via) { if !e.1.is_empty() { e.1.push('+'); } e.1.push_str(via); }
        }
    };
    match mode {
        "grep" => add(grep_scan(idx, query, k * 2), "grep", 1.0),
        "bm25" => add(bm25(idx, &q, k * 2), "bm25", 1.0),
        "vector" => add(vector(idx, query, k * 2), "vector", 1.0),
        _ => {
            // selectors (IPs, emails, hashes, domains) get an exact grep boost
            for term in query.split_whitespace().filter(|t| t.len() >= 4 && (t.contains('.') || t.contains('@') || t.chars().all(|c| c.is_ascii_hexdigit()))) {
                add(grep_scan(idx, term, k), "grep", 1.4);
            }
            add(bm25(idx, &q, k * 2), "bm25", 1.0);
            add(vector(idx, query, k * 2), "vector", 1.1);
        }
    }
    let mut hits: Vec<Hit> = fused.into_iter().map(|(i, (s, via))| { let d = &idx.docs[i]; Hit { id: d.id.clone(), kind: d.kind.clone(), title: d.title.clone(), text: d.text.chars().take(600).collect(), score: (s * 1000.0).round() / 1000.0, entity_ids: d.entity_ids.clone(), via } }).collect();
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(k);
    bus::emit("memory.retrieve", format!("\"{}\" → {} hits ({})", query.chars().take(60).collect::<String>(), hits.len(), mode));
    hits
}

/// GraphRAG context: retrieve, expand one hop, pack into a bounded block.
/// Returns (context_text, entity_ids_involved).
pub fn build_context(project: &str, question: &str, graph: Option<&serde_json::Value>, max_chars: usize) -> (String, Vec<String>) {
    let hits = search(project, question, 14, graph, "hybrid");
    let m = indexes().lock().unwrap();
    let Some(idx) = m.get(project) else { return (String::new(), vec![]) };
    let mut seed: Vec<String> = Vec::new();
    for h in &hits { for id in &h.entity_ids { if idx.entities.contains_key(id) && !seed.contains(id) { seed.push(id.clone()); } } }
    seed.truncate(10);
    let mut involved: Vec<String> = seed.clone();
    let mut out = String::new();
    out.push_str("CONTEXTO RECUPERADO (GraphRAG — só o relevante para a pergunta):\n\nENTIDADES:\n");
    for id in &seed {
        if let Some(d) = idx.docs.iter().find(|d| d.id == *id) {
            out.push_str(&format!("- {}\n", d.text.chars().take(280).collect::<String>()));
        }
    }
    out.push_str("\nVIZINHANÇA (1 salto):\n");
    let mut lines = 0;
    for id in &seed {
        if let Some(nb) = idx.adj.get(id) {
            let (sl, _) = idx.entities.get(id).cloned().unwrap_or((id.clone(), String::new()));
            for (other, ty, outgoing) in nb.iter().take(12) {
                let (ol, ok) = idx.entities.get(other).cloned().unwrap_or((other.clone(), "?".into()));
                out.push_str(&if *outgoing { format!("- {sl} --{}--> {ol} [{ok}]\n", ty.replace('_', " ")) } else { format!("- {ol} [{ok}] --{}--> {sl}\n", ty.replace('_', " ")) });
                if !involved.contains(other) { involved.push(other.clone()); }
                lines += 1;
                if lines > 80 { break; }
            }
        }
    }
    let others: Vec<&Hit> = hits.iter().filter(|h| h.kind != "entity" && h.kind != "relationship").collect();
    if !others.is_empty() {
        out.push_str("\nMEMÓRIA / NOTAS / BRIEF:\n");
        for h in others.iter().take(8) { out.push_str(&format!("- [{}] {}: {}\n", h.kind, h.title, h.text.chars().take(320).collect::<String>())); }
    }
    if seed.is_empty() && others.is_empty() {
        bus::emit("memory.context", "no relevant context — falling back to compact graph listing");
        return (String::new(), vec![]);
    }
    let packed = prep::compact(&out, max_chars);
    bus::emit("memory.context", format!("{} seeds · {} involved · {} chars", seed.len(), involved.len(), packed.chars().count()));
    (packed, involved)
}

pub fn stats(project: &str) -> serde_json::Value {
    let m = indexes().lock().unwrap();
    let (docs, ents) = m.get(project).map(|i| (i.docs.len(), i.entities.len())).unwrap_or((0, 0));
    serde_json::json!({ "facts": facts(project).len(), "indexed_docs": docs, "indexed_entities": ents, "embeddings": crate::embed::is_configured() })
}
