//! Link prediction: infer edges that are LIKELY but ABSENT — "these two entities
//! are probably connected even though there's no direct link". This is the
//! strongest sense of "potentiate": inferring what the data doesn't state.
//!
//! Deterministic and offline, using classic topological predictors over the
//! undirected graph — common neighbours + Adamic-Adar (shared neighbours weighted
//! by rarity: a link through a niche hub counts more than through a giant one).
//! Predictions are added as `predicted_link` relationships marked `predicted`, so
//! they are visibly distinct from observed edges and never mistaken for fact.

use crate::ontology::{KnowledgeGraph, Relationship};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Prediction {
    pub a: String,
    pub b: String,
    /// Adamic-Adar score (higher = stronger structural signal).
    pub score: f32,
    /// Number of shared neighbours.
    pub common: usize,
}

/// Minimum shared neighbours for a pair to be a candidate prediction.
pub const DEFAULT_MIN_SHARED: usize = 2;

/// Predict likely-but-absent links. Honours CORTEX_LINK_MIN_SHARED (tunable
/// without a rebuild); falls back to the default.
pub fn predict(graph: &KnowledgeGraph, top_k: usize) -> Vec<Prediction> {
    let min_shared = std::env::var("CORTEX_LINK_MIN_SHARED").ok().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_MIN_SHARED);
    predict_with(graph, min_shared.max(1), top_k)
}

/// Predict likely-but-absent links, ranked by Adamic-Adar, requiring at least
/// `min_shared` shared neighbours (a single shared hub is too weak).
pub fn predict_with(graph: &KnowledgeGraph, min_shared: usize, top_k: usize) -> Vec<Prediction> {
    let ids: Vec<&String> = graph.entities.keys().collect();
    let n = ids.len();
    if n < 3 {
        return Vec::new();
    }
    let index: HashMap<&str, usize> = ids.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();

    // Undirected adjacency sets.
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for r in &graph.relationships {
        // Never let predictions feed predictions — only observed edges count.
        if r.rel_type == "predicted_link" {
            continue;
        }
        let (Some(&a), Some(&b)) = (index.get(r.source_id.as_str()), index.get(r.target_id.as_str())) else { continue };
        if a != b {
            adj[a].insert(b);
            adj[b].insert(a);
        }
    }

    // Accumulate common-neighbour count + Adamic-Adar over each shared node z.
    // Skip giant hubs as common neighbours (they'd link everything spuriously and
    // add near-zero AA weight anyway).
    let mut common: HashMap<(usize, usize), usize> = HashMap::new();
    let mut aa: HashMap<(usize, usize), f32> = HashMap::new();
    for z in 0..n {
        let d = adj[z].len();
        if d < 2 || d > 25 {
            continue;
        }
        let w = 1.0 / (d as f32).ln();
        let mut nbrs: Vec<usize> = adj[z].iter().copied().collect();
        nbrs.sort_unstable();
        for i in 0..nbrs.len() {
            for j in (i + 1)..nbrs.len() {
                let key = (nbrs[i], nbrs[j]);
                *common.entry(key).or_insert(0) += 1;
                *aa.entry(key).or_insert(0.0) += w;
            }
        }
    }

    // Keep pairs with >=2 shared neighbours that are NOT already directly linked.
    let mut preds: Vec<Prediction> = common
        .into_iter()
        .filter(|(pair, c)| *c >= min_shared && !adj[pair.0].contains(&pair.1))
        .map(|((u, v), c)| Prediction {
            a: ids[u].clone(),
            b: ids[v].clone(),
            score: *aa.get(&(u, v)).unwrap_or(&0.0),
            common: c,
        })
        .collect();

    preds.sort_by(|x, y| y.score.partial_cmp(&x.score).unwrap().then(y.common.cmp(&x.common)));
    preds.truncate(top_k);
    preds
}

/// Add predictions to the graph as `predicted_link` relationships (marked
/// `predicted=true`), so the GUI can render them distinctly (e.g. dashed) and
/// nothing downstream treats them as observed fact.
pub fn add_to_graph(graph: &mut KnowledgeGraph, preds: &[Prediction]) {
    if preds.is_empty() {
        return;
    }
    let max = preds.iter().map(|p| p.score).fold(0.0f32, f32::max).max(1e-6);
    for p in preds {
        // Map AA to a modest confidence band [0.3, 0.7] — a prediction, never fact.
        let conf = 0.3 + (p.score / max) * 0.4;
        let mut r = Relationship::new(p.a.clone(), "predicted_link", p.b.clone(), conf);
        r.source_reference = Some(format!("inferred:link-prediction ({} shared neighbours)", p.common));
        r.attributes.insert("predicted".into(), "true".into());
        r.attributes.insert("shared_neighbors".into(), p.common.to_string());
        graph.add_relationship(r);
    }
}
