//! Attack-path analysis on the knowledge graph.
//!
//! Weighted shortest paths between two entities, using edge confidence as the
//! link strength (weight = 1 − confidence, so high-confidence edges are cheap
//! to traverse). Supports:
//!   * single shortest path (Dijkstra)
//!   * K shortest paths (Yen's algorithm)
//!   * path scoring (product of edge confidences along the route)
//!
//! Results describe each hop with the entities, relationship types, and per-edge
//! confidence so the GUI can render a scored attack chain.

use crate::ontology::KnowledgeGraph;
use serde::Serialize;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize)]
pub struct AttackPath {
    pub hops: Vec<Hop>,
    pub total_score: f32,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hop {
    pub from_id: String,
    pub from_label: String,
    pub from_kind: String,
    pub to_id: String,
    pub to_label: String,
    pub to_kind: String,
    pub rel_type: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttackPathResult {
    pub paths: Vec<AttackPath>,
    pub source_id: String,
    pub target_id: String,
}

#[derive(Clone, PartialEq)]
struct State {
    cost: f64,
    node: usize,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
struct Edge {
    to: usize,
    weight: f64,
    confidence: f32,
    rel_type: String,
}

pub fn find_paths(
    graph: &KnowledgeGraph,
    source_id: &str,
    target_id: &str,
    max_paths: usize,
    max_depth: usize,
) -> AttackPathResult {
    let ids: Vec<String> = graph.entities.keys().cloned().collect();
    let n = ids.len();
    let index: HashMap<&str, usize> = ids.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();

    let src = index.get(source_id).copied();
    let tgt = index.get(target_id).copied();

    if src.is_none() || tgt.is_none() || n == 0 {
        return AttackPathResult { paths: vec![], source_id: source_id.into(), target_id: target_id.into() };
    }
    let src = src.unwrap();
    let tgt = tgt.unwrap();

    // Build weighted directed adjacency. Keep all edges (both directions for
    // undirected traversal) with the best confidence per (from, to) pair.
    let mut adj: Vec<Vec<Edge>> = vec![Vec::new(); n];
    let mut best: HashMap<(usize, usize), (f32, String)> = HashMap::new();
    for r in &graph.relationships {
        let (Some(&a), Some(&b)) = (index.get(r.source_id.as_str()), index.get(r.target_id.as_str())) else { continue };
        if a == b { continue; }
        let conf = r.confidence.clamp(0.01, 1.0);
        for (from, to) in [(a, b), (b, a)] {
            let entry = best.entry((from, to)).or_insert((0.0, String::new()));
            if conf > entry.0 {
                *entry = (conf, r.rel_type.clone());
            }
        }
    }
    for ((from, to), (conf, rel)) in &best {
        let weight = 1.0 - (*conf as f64) + 0.01; // small epsilon so zero-cost edges don't collapse
        adj[*from].push(Edge { to: *to, weight, confidence: *conf, rel_type: rel.clone() });
    }

    let k = max_paths.min(10).max(1);
    let paths = yen_k_shortest(&adj, src, tgt, k, max_depth);

    let result_paths: Vec<AttackPath> = paths.into_iter().map(|node_path| {
        build_attack_path(graph, &ids, &adj, &node_path)
    }).collect();

    AttackPathResult { paths: result_paths, source_id: source_id.into(), target_id: target_id.into() }
}

fn dijkstra(
    adj: &[Vec<Edge>],
    src: usize,
    tgt: usize,
    max_depth: usize,
    blocked_edges: &HashSet<(usize, usize)>,
) -> Option<Vec<usize>> {
    let n = adj.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    let mut depth = vec![0usize; n];
    let mut heap = BinaryHeap::new();

    dist[src] = 0.0;
    heap.push(State { cost: 0.0, node: src });

    while let Some(State { cost, node }) = heap.pop() {
        if node == tgt {
            let mut path = vec![tgt];
            let mut cur = tgt;
            while let Some(p) = prev[cur] {
                path.push(p);
                cur = p;
            }
            path.reverse();
            return Some(path);
        }
        if cost > dist[node] { continue; }
        if depth[node] >= max_depth { continue; }

        for edge in &adj[node] {
            if blocked_edges.contains(&(node, edge.to)) { continue; }
            let next_cost = cost + edge.weight;
            if next_cost < dist[edge.to] {
                dist[edge.to] = next_cost;
                prev[edge.to] = Some(node);
                depth[edge.to] = depth[node] + 1;
                heap.push(State { cost: next_cost, node: edge.to });
            }
        }
    }
    None
}

fn yen_k_shortest(
    adj: &[Vec<Edge>],
    src: usize,
    tgt: usize,
    k: usize,
    max_depth: usize,
) -> Vec<Vec<usize>> {
    let blocked = HashSet::new();
    let Some(first) = dijkstra(adj, src, tgt, max_depth, &blocked) else {
        return vec![];
    };

    let mut a: Vec<Vec<usize>> = vec![first];
    let mut b: Vec<(f64, Vec<usize>)> = Vec::new();

    for ki in 1..k {
        let prev_path = &a[ki - 1];
        for i in 0..prev_path.len().saturating_sub(1) {
            let spur_node = prev_path[i];
            let root_path: Vec<usize> = prev_path[..=i].to_vec();

            // Block edges used by existing paths that share the same root.
            let mut blocked = HashSet::new();
            for p in &a {
                if p.len() > i && p[..=i] == root_path[..] {
                    blocked.insert((p[i], p[i + 1]));
                }
            }
            // Block root path nodes (except spur) to avoid loops.
            for &node in &root_path[..i] {
                for edge in &adj[node] {
                    blocked.insert((node, edge.to));
                }
                for neighbor in &adj[node] {
                    blocked.insert((neighbor.to, node));
                }
            }

            if let Some(spur_path) = dijkstra(adj, spur_node, tgt, max_depth - i, &blocked) {
                let mut total = root_path[..i].to_vec();
                total.extend_from_slice(&spur_path);
                let cost = path_cost(adj, &total);
                if !a.contains(&total) && !b.iter().any(|(_, p)| p == &total) {
                    b.push((cost, total));
                }
            }
        }

        if b.is_empty() { break; }
        b.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        let (_, best) = b.remove(0);
        a.push(best);
    }

    a
}

fn path_cost(adj: &[Vec<Edge>], path: &[usize]) -> f64 {
    let mut cost = 0.0;
    for w in path.windows(2) {
        let from = w[0]; let to = w[1];
        if let Some(e) = adj[from].iter().find(|e| e.to == to) {
            cost += e.weight;
        } else {
            cost += 100.0; // should not happen
        }
    }
    cost
}

fn build_attack_path(
    graph: &KnowledgeGraph,
    ids: &[String],
    adj: &[Vec<Edge>],
    node_path: &[usize],
) -> AttackPath {
    let mut hops = Vec::new();
    let mut score: f64 = 1.0;

    for w in node_path.windows(2) {
        let from_idx = w[0];
        let to_idx = w[1];
        let from_id = &ids[from_idx];
        let to_id = &ids[to_idx];

        let (conf, rel) = if let Some(e) = adj[from_idx].iter().find(|e| e.to == to_idx) {
            (e.confidence, e.rel_type.clone())
        } else {
            (0.5, "unknown".to_string())
        };

        let from_ent = graph.entities.get(from_id);
        let to_ent = graph.entities.get(to_id);

        hops.push(Hop {
            from_id: from_id.clone(),
            from_label: from_ent.map(|e| e.label.clone()).unwrap_or_default(),
            from_kind: from_ent.map(|e| e.kind.as_str().to_string()).unwrap_or_default(),
            to_id: to_id.clone(),
            to_label: to_ent.map(|e| e.label.clone()).unwrap_or_default(),
            to_kind: to_ent.map(|e| e.kind.as_str().to_string()).unwrap_or_default(),
            rel_type: rel,
            confidence: conf,
        });

        score *= conf as f64;
    }

    AttackPath {
        length: hops.len(),
        total_score: score as f32,
        hops,
    }
}
