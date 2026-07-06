//! Graph correlation: derive cross-record relationships that intra-record
//! extraction can't see. The core move is "shared hub" detection — two accounts
//! that both logged in from the same IP become `same_ip_as` peers, etc.

use crate::ontology::{EntityKind, KnowledgeGraph, Relationship};
use std::collections::HashMap;

/// Add correlation edges to the graph. Returns the number of edges added.
pub fn correlate(graph: &mut KnowledgeGraph) -> usize {
    let before = graph.relationship_count();

    // hub entity id -> list of neighbor entity ids that connect to it.
    let mut hub_neighbors: HashMap<String, Vec<String>> = HashMap::new();
    // Kinds that act as shared hubs and the peer relationship they imply.
    let peer_rel = |kind: EntityKind| -> Option<&'static str> {
        match kind {
            EntityKind::Ip => Some("same_ip_as"),
            EntityKind::Device => Some("same_device_as"),
            EntityKind::Wallet => Some("shares_wallet_with"),
            EntityKind::Domain => Some("shares_domain_with"),
            EntityKind::Group => Some("co_member_with"),
            _ => None,
        }
    };

    // Snapshot relationships to avoid borrow conflicts.
    let rels: Vec<(String, String)> = graph
        .relationships
        .iter()
        .map(|r| (r.source_id.clone(), r.target_id.clone()))
        .collect();

    for (src, tgt) in &rels {
        // Record neighbor→hub in both directions; hub is whichever end is a hub kind.
        if let Some(t) = graph.entities.get(tgt) {
            if peer_rel(t.kind).is_some() {
                hub_neighbors.entry(tgt.clone()).or_default().push(src.clone());
            }
        }
        if let Some(s) = graph.entities.get(src) {
            if peer_rel(s.kind).is_some() {
                hub_neighbors.entry(src.clone()).or_default().push(tgt.clone());
            }
        }
    }

    let mut new_edges: Vec<Relationship> = Vec::new();
    for (hub, neighbors) in &hub_neighbors {
        let Some(hub_e) = graph.entities.get(hub) else { continue };
        let Some(rel) = peer_rel(hub_e.kind) else { continue };
        // Distinct neighbors.
        let mut uniq: Vec<&String> = neighbors.iter().collect();
        uniq.sort();
        uniq.dedup();
        // Pairwise peer links (cap to keep the graph readable on big hubs).
        for i in 0..uniq.len() {
            for j in (i + 1)..uniq.len().min(i + 25) {
                let mut r = Relationship::new(uniq[i].clone(), rel, uniq[j].clone(), 0.5);
                r.source_reference = Some(format!("shared:{}", hub_e.label));
                new_edges.push(r);
            }
        }
    }

    for e in new_edges {
        graph.add_relationship(e);
    }

    graph.relationship_count() - before
}

/// Merge LLM-proposed relationships (by entity id) into the graph.
pub fn merge_llm(graph: &mut KnowledgeGraph, llm: &serde_json::Value) -> usize {
    let before = graph.relationship_count();
    let Some(arr) = llm.get("relationships").and_then(|v| v.as_array()) else {
        return 0;
    };
    for item in arr {
        let (Some(s), Some(rel), Some(t)) = (
            item.get("source").and_then(|v| v.as_str()),
            item.get("type").and_then(|v| v.as_str()),
            item.get("target").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        // Only accept ids that exist in the graph (no invented nodes).
        if !graph.entities.contains_key(s) || !graph.entities.contains_key(t) {
            continue;
        }
        let conf = item.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
        let mut r = Relationship::new(s, rel, t, conf);
        if let Some(ev) = item.get("evidence").and_then(|v| v.as_str()) {
            r.source_reference = Some(format!("llm:{ev}"));
        } else {
            r.source_reference = Some("llm".into());
        }
        graph.add_relationship(r);
    }
    graph.relationship_count() - before
}
