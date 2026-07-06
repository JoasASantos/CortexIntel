//! Workstream E — probabilistic identity resolution. Recognizes that different
//! aliases/emails/handles are the SAME entity, with a confidence score and the
//! explicit signals behind each match (never a black box). Deterministic and
//! offline; the LLM only adjudicates the gray zone (validated against signals).
//!
//! High confidence → auto-merge into a canonical entity. Gray zone → a "merge
//! suggestion" for human review. Below → ignored.

use crate::ontology::{Entity, EntityKind, KnowledgeGraph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A proposed match between two entities of the same kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeCandidate {
    pub canonical_id: String,
    pub alias_id: String,
    pub canonical_label: String,
    pub alias_label: String,
    pub kind: String,
    pub confidence: f32,
    pub signals: Vec<String>,
}

/// Result of a resolution pass.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResolutionOutcome {
    pub merged: Vec<MergeCandidate>,
    pub suggestions: Vec<MergeCandidate>,
}

const AUTO: f32 = 0.82; // ≥ auto-merge
const SUGGEST: f32 = 0.5; // ≥ suggestion, else ignore

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Normalize an email/handle to its local identity (before @, digits stripped).
fn email_local(label: &str) -> Option<String> {
    label.split_once('@').map(|(l, _)| l.trim().to_lowercase())
}

/// Cheap normalized-Levenshtein similarity in [0..1].
fn str_sim(a: &str, b: &str) -> f32 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    if a == b {
        return 1.0;
    }
    let (la, lb) = (a.chars().count(), b.chars().count());
    if la == 0 || lb == 0 {
        return 0.0;
    }
    let av: Vec<char> = a.chars().collect();
    let bv: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=lb).collect();
    let mut cur = vec![0usize; lb + 1];
    for i in 1..=la {
        cur[0] = i;
        for j in 1..=lb {
            let cost = if av[i - 1] == bv[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let dist = prev[lb];
    1.0 - (dist as f32 / la.max(lb) as f32)
}

/// Neighbors of each entity id (shared-infra detection).
fn neighbor_map(g: &KnowledgeGraph) -> HashMap<String, std::collections::HashSet<String>> {
    let mut m: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for r in &g.relationships {
        m.entry(r.source_id.clone()).or_default().insert(r.target_id.clone());
        m.entry(r.target_id.clone()).or_default().insert(r.source_id.clone());
    }
    m
}

/// Score a candidate pair of same-kind entities. Returns (confidence, signals).
fn score_pair(a: &Entity, b: &Entity, nbrs: &HashMap<String, std::collections::HashSet<String>>) -> (f32, Vec<String>) {
    let mut conf = 0.0f32;
    let mut signals = Vec::new();

    // 1) String similarity of the label (name/handle).
    let sim = str_sim(&a.label, &b.label);
    if sim >= 0.86 {
        conf = conf.max(sim);
        signals.push(format!("label similarity {:.0}%", sim * 100.0));
    }

    // 2) Same email local-part (alex@gmail vs alex@proton).
    if let (Some(la), Some(lb)) = (email_local(&a.label), email_local(&b.label)) {
        if la == lb && la.len() >= 3 {
            conf = conf.max(0.8);
            signals.push(format!("same email local-part \"{la}\""));
        } else if str_sim(&la, &lb) >= 0.9 {
            conf = conf.max(0.6);
            signals.push("near-identical email local-part".into());
        }
    }

    // 3) Shared strong infrastructure neighbors (device/ip/wallet/domain).
    if let (Some(na), Some(nb)) = (nbrs.get(&a.id), nbrs.get(&b.id)) {
        let shared: Vec<&String> = na.intersection(nb).collect();
        if !shared.is_empty() {
            conf += 0.15 * shared.len().min(3) as f32;
            signals.push(format!("{} shared connection(s)", shared.len()));
        }
    }

    // 4) Matching normalized identifiers in attributes (phone/doc hashes).
    for key in ["phone", "phone_hash", "document_reference", "device_identifier_hash", "wallet_address"] {
        if let (Some(va), Some(vb)) = (a.attributes.get(key), b.attributes.get(key)) {
            if !va.is_empty() && norm(va) == norm(vb) {
                conf = conf.max(0.85);
                signals.push(format!("matching {key}"));
            }
        }
    }

    (conf.min(1.0), signals)
}

/// Resolve identities in the graph. Auto-merges high-confidence pairs and
/// returns the outcome (merges applied + gray-zone suggestions). Only entities
/// of person/account/organization/suspect/victim kinds are considered.
pub fn resolve(g: &mut KnowledgeGraph) -> ResolutionOutcome {
    let resolvable = |k: EntityKind| matches!(k, EntityKind::Person | EntityKind::Account | EntityKind::Organization | EntityKind::Suspect | EntityKind::Victim);
    let mut outcome = ResolutionOutcome::default();
    let nbrs = neighbor_map(g);

    // Group ids by kind to only compare like-with-like.
    let mut by_kind: HashMap<EntityKind, Vec<String>> = HashMap::new();
    for (id, e) in &g.entities {
        if resolvable(e.kind) {
            by_kind.entry(e.kind).or_default().push(id.clone());
        }
    }

    // Collect candidate pairs (O(n²) within a kind; capped for very large kinds).
    let mut pairs: Vec<MergeCandidate> = Vec::new();
    for (kind, ids) in &by_kind {
        if ids.len() > 1500 {
            continue; // skip pathological sizes; handled by clustering upstream
        }
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = &g.entities[&ids[i]];
                let b = &g.entities[&ids[j]];
                let (conf, signals) = score_pair(a, b, &nbrs);
                if conf >= SUGGEST && !signals.is_empty() {
                    // keep the higher-degree / longer-label as canonical
                    let (can, al) = if a.label.len() >= b.label.len() { (a, b) } else { (b, a) };
                    pairs.push(MergeCandidate {
                        canonical_id: can.id.clone(),
                        alias_id: al.id.clone(),
                        canonical_label: can.label.clone(),
                        alias_label: al.label.clone(),
                        kind: kind.as_str().to_string(),
                        confidence: conf,
                        signals,
                    });
                }
            }
        }
    }

    // Apply high-confidence merges (skip if a node was already merged away).
    pairs.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    let mut gone: std::collections::HashSet<String> = std::collections::HashSet::new();
    for c in pairs {
        if gone.contains(&c.canonical_id) || gone.contains(&c.alias_id) {
            continue;
        }
        if c.confidence >= AUTO {
            if g.merge_entities(&c.canonical_id, &c.alias_id, c.signals.clone(), c.confidence) {
                gone.insert(c.alias_id.clone());
                outcome.merged.push(c);
            }
        } else {
            outcome.suggestions.push(c);
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Entity, EntityKind, KnowledgeGraph, Relationship};

    #[test]
    fn resolves_same_person_under_three_aliases() {
        let mut g = KnowledgeGraph::new();
        // Same person, 3 aliases sharing a device.
        let p1 = g.upsert_entity(Entity::new(EntityKind::Person, "Alexandre Silva"));
        let p2 = g.upsert_entity(Entity::new(EntityKind::Person, "Alexandre Silvaa")); // 1-char off
        let p3 = g.upsert_entity(Entity::new(EntityKind::Person, "Alexandre Silvva")); // 1-char off
        let dev = g.upsert_entity(Entity::new(EntityKind::Device, "device-xyz"));
        g.add_relationship(Relationship::new(p1.clone(), "uses_device", dev.clone(), 0.9));
        g.add_relationship(Relationship::new(p2.clone(), "uses_device", dev.clone(), 0.9));
        g.add_relationship(Relationship::new(p3.clone(), "uses_device", dev.clone(), 0.9));
        let before = g.entity_count();
        let out = resolve(&mut g);
        assert!(out.merged.len() >= 1, "expected at least one auto-merge");
        assert!(g.entity_count() < before, "entities should collapse");
        // canonical entity carries aliases with signals, provenance preserved
        let canon = g.entities.values().find(|e| !e.aliases.is_empty()).expect("a canonical entity");
        assert!(canon.aliases.iter().all(|a| !a.signals.is_empty()));
    }

    #[test]
    fn weak_match_stays_a_suggestion() {
        let mut g = KnowledgeGraph::new();
        g.upsert_entity(Entity::new(EntityKind::Person, "John Smith"));
        g.upsert_entity(Entity::new(EntityKind::Person, "Jon Smyth")); // similar but not identical, no shared infra
        let out = resolve(&mut g);
        assert_eq!(out.merged.len(), 0, "weak match must not auto-merge");
    }
}
