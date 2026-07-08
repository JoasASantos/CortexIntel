//! Network science over the knowledge graph: structural intelligence that plain
//! correlation can't give. Three deterministic, offline analyses on the
//! undirected view of the graph:
//!   * betweenness centrality (Brandes) — the BROKER: the node whose removal most
//!     fragments the network (a kingpin / chokepoint).
//!   * PageRank (power iteration) — global importance by who-points-to-whom.
//!   * community detection (label propagation) — emergent clusters + modularity.
//! Results are written back as entity attributes/tags so risk, assessment and the
//! graph view can use them. No new dependencies; runs on the in-memory graph.

use crate::ontology::KnowledgeGraph;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct NetworkAnalysis {
    /// entity id -> normalized betweenness [0..1]
    pub betweenness: HashMap<String, f32>,
    /// entity id -> normalized pagerank [0..1]
    pub pagerank: HashMap<String, f32>,
    /// entity id -> community id
    pub community: HashMap<String, usize>,
    /// number of communities found
    pub communities: usize,
    /// modularity of the partition [-0.5..1]
    pub modularity: f32,
    /// entity id of the top broker (max betweenness), if any
    pub top_broker: Option<String>,
}

/// Analyze the graph's structure. Operates on the undirected, simple view.
pub fn analyze(graph: &KnowledgeGraph) -> NetworkAnalysis {
    let ids: Vec<String> = graph.entities.keys().cloned().collect();
    let n = ids.len();
    let mut out = NetworkAnalysis::default();
    if n == 0 {
        return out;
    }
    let index: HashMap<&str, usize> = ids.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();

    // Undirected simple adjacency (dedup, no self-loops).
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    {
        let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for r in &graph.relationships {
            let (Some(&a), Some(&b)) = (index.get(r.source_id.as_str()), index.get(r.target_id.as_str())) else { continue };
            if a == b {
                continue;
            }
            let key = if a < b { (a, b) } else { (b, a) };
            if seen.insert(key) {
                adj[a].push(b);
                adj[b].push(a);
            }
        }
    }

    let bet = brandes_betweenness(&adj);
    let pr = pagerank(&adj, 0.85, 60);
    let (comm, k) = label_propagation(&adj);
    let modularity = modularity(&adj, &comm);

    // Normalize centralities to [0..1] for consistency with the rest of the app.
    let norm = |v: &[f32]| -> Vec<f32> {
        let max = v.iter().cloned().fold(0.0f32, f32::max);
        if max <= 0.0 { v.to_vec() } else { v.iter().map(|x| x / max).collect() }
    };
    let betn = norm(&bet);
    let prn = norm(&pr);

    let mut top_broker: Option<(usize, f32)> = None;
    for (i, id) in ids.iter().enumerate() {
        out.betweenness.insert(id.clone(), betn[i]);
        out.pagerank.insert(id.clone(), prn[i]);
        out.community.insert(id.clone(), comm[i]);
        if betn[i] > 0.0 && top_broker.map(|(_, b)| betn[i] > b).unwrap_or(true) {
            top_broker = Some((i, betn[i]));
        }
    }
    out.communities = k;
    out.modularity = modularity;
    out.top_broker = top_broker.map(|(i, _)| ids[i].clone());
    out
}

/// Brandes' algorithm for betweenness centrality (unweighted, undirected).
fn brandes_betweenness(adj: &[Vec<usize>]) -> Vec<f32> {
    let n = adj.len();
    let mut cb = vec![0.0f64; n];
    for s in 0..n {
        let mut stack: Vec<usize> = Vec::new();
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut sigma = vec![0.0f64; n];
        let mut dist = vec![-1i64; n];
        sigma[s] = 1.0;
        dist[s] = 0;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(s);
        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for &w in &adj[v] {
                if dist[w] < 0 {
                    dist[w] = dist[v] + 1;
                    queue.push_back(w);
                }
                if dist[w] == dist[v] + 1 {
                    sigma[w] += sigma[v];
                    preds[w].push(v);
                }
            }
        }
        let mut delta = vec![0.0f64; n];
        while let Some(w) = stack.pop() {
            for &v in &preds[w] {
                delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
            }
            if w != s {
                cb[w] += delta[w];
            }
        }
    }
    // Undirected: each shortest path counted twice.
    cb.iter().map(|x| (*x as f32) / 2.0).collect()
}

/// PageRank via power iteration on the undirected graph (edges both ways).
fn pagerank(adj: &[Vec<usize>], damping: f32, iters: usize) -> Vec<f32> {
    let n = adj.len();
    if n == 0 {
        return Vec::new();
    }
    let mut pr = vec![1.0f32 / n as f32; n];
    let deg: Vec<f32> = adj.iter().map(|a| a.len() as f32).collect();
    let base = (1.0 - damping) / n as f32;
    for _ in 0..iters {
        let mut next = vec![base; n];
        // dangling nodes (degree 0) redistribute their mass uniformly.
        let dangling: f32 = (0..n).filter(|&i| deg[i] == 0.0).map(|i| pr[i]).sum::<f32>() / n as f32;
        for v in 0..n {
            for &u in &adj[v] {
                next[v] += damping * pr[u] / deg[u];
            }
            next[v] += damping * dangling;
        }
        pr = next;
    }
    pr
}

/// Community detection by label propagation. Deterministic: fixed node order
/// (insertion order) and smallest-label tie-breaking. Returns (labels, count).
fn label_propagation(adj: &[Vec<usize>]) -> (Vec<usize>, usize) {
    let n = adj.len();
    let mut label: Vec<usize> = (0..n).collect();
    for _ in 0..20 {
        let mut changed = false;
        for v in 0..n {
            if adj[v].is_empty() {
                continue;
            }
            // Most frequent neighbor label; ties → smallest label (deterministic).
            let mut counts: HashMap<usize, usize> = HashMap::new();
            for &u in &adj[v] {
                *counts.entry(label[u]).or_insert(0) += 1;
            }
            let best = counts
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(l, _)| *l)
                .unwrap_or(label[v]);
            if best != label[v] {
                label[v] = best;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Renumber labels to 0..k.
    let mut remap: HashMap<usize, usize> = HashMap::new();
    for l in &mut label {
        let next = remap.len();
        *l = *remap.entry(*l).or_insert(next);
    }
    let k = remap.len();
    (label, k)
}

/// Newman modularity Q of a partition on the undirected graph.
fn modularity(adj: &[Vec<usize>], comm: &[usize]) -> f32 {
    let m: f32 = adj.iter().map(|a| a.len() as f32).sum::<f32>() / 2.0;
    if m == 0.0 {
        return 0.0;
    }
    let deg: Vec<f32> = adj.iter().map(|a| a.len() as f32).collect();
    let mut q = 0.0f32;
    for v in 0..adj.len() {
        for w in 0..adj.len() {
            if comm[v] != comm[w] {
                continue;
            }
            let a_vw = if adj[v].contains(&w) { 1.0 } else { 0.0 };
            q += a_vw - (deg[v] * deg[w]) / (2.0 * m);
        }
    }
    q / (2.0 * m)
}
