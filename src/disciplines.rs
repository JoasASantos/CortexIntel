//! Intelligence-discipline signals (v0.0.2): deterministic, offline analytics
//! specific to HUMINT, SIGINT and OSINT, layered on the common engine. Each is a
//! correlation/scoring pass that runs in the pipeline and feeds risk + the graph.
//!
//!  * HUMINT — grade source/report reliability with the NATO Admiralty Code
//!    (source A–F × info 1–6), from declared reliability/corroboration signals.
//!  * SIGINT — communication-pattern correlation: link entities that co-occur on
//!    the same communications (a comms analogue of the shared-hub correlator),
//!    with privacy minimization (metadata only).
//!  * OSINT — selector/handle reuse: the same username/handle across sources is a
//!    strong same-actor candidate.

use crate::ontology::{EntityKind, KnowledgeGraph, Relationship};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct DisciplineStats {
    pub humint_graded: usize,
    pub sigint_links: usize,
    pub osint_links: usize,
}

pub fn apply(graph: &mut KnowledgeGraph) -> DisciplineStats {
    let mut s = DisciplineStats::default();
    s.humint_graded = humint_grade(graph);
    s.sigint_links = sigint_comms(graph);
    s.osint_links = osint_selectors(graph);
    s
}

// ---------------------------------------------------------------------------
// HUMINT — Admiralty Code reliability grading
// ---------------------------------------------------------------------------
/// Map a free-text reliability hint to the Admiralty source grade A–F.
fn source_grade(v: &str) -> Option<char> {
    let l = v.to_lowercase();
    if l.contains("completely reliable") || l.contains("totalmente confiá") || l == "a" { return Some('A'); }
    if l.contains("usually reliable") || l.contains("confiá") || l == "b" { return Some('B'); }
    if l.contains("fairly reliable") || l == "c" { return Some('C'); }
    if l.contains("not usually reliable") || l.contains("pouco confiá") || l == "d" { return Some('D'); }
    if l.contains("unreliable") || l.contains("não confiá") || l == "e" { return Some('E'); }
    if l.contains("cannot be judged") || l.contains("desconhec") || l == "f" { return Some('F'); }
    None
}
/// Map a credibility/corroboration hint to the Admiralty info grade 1–6.
fn info_grade(v: &str) -> Option<char> {
    let l = v.to_lowercase();
    if l.contains("confirmed") || l.contains("confirmad") || l == "1" { return Some('1'); }
    if l.contains("probably true") || l.contains("provavelmente") || l == "2" { return Some('2'); }
    if l.contains("possibly true") || l.contains("possivelmente") || l == "3" { return Some('3'); }
    if l.contains("doubtful") || l.contains("duvidos") || l == "4" { return Some('4'); }
    if l.contains("improbable") || l.contains("imprová") || l == "5" { return Some('5'); }
    if l.contains("cannot be judged") || l.contains("não avaliad") || l == "6" { return Some('6'); }
    None
}

fn humint_grade(graph: &mut KnowledgeGraph) -> usize {
    let ids: Vec<String> = graph.entities.keys().cloned().collect();
    let mut n = 0;
    for id in ids {
        let e = graph.entities.get(&id).unwrap();
        if !matches!(e.kind, EntityKind::Report | EntityKind::Communication) {
            continue;
        }
        // Look for reliability + credibility hints in attributes.
        let mut src = e.attributes.iter().find(|(k, _)| { let lk = k.to_lowercase(); lk.contains("reliab") || lk.contains("confiab") || lk.contains("source_grade") }).and_then(|(_, v)| source_grade(v));
        let mut inf = e.attributes.iter().find(|(k, _)| { let lk = k.to_lowercase(); lk.contains("credib") || lk.contains("confirm") || lk.contains("info_grade") }).and_then(|(_, v)| info_grade(v));
        // Corroboration fallback: an info grade from how many sources it has.
        if inf.is_none() {
            inf = Some(match e.sources.len() { 0 | 1 => '3', 2 => '2', _ => '1' });
        }
        if src.is_none() {
            src = Some('F'); // unknown source reliability
        }
        let (sg, ig) = (src.unwrap(), inf.unwrap());
        let e = graph.entities.get_mut(&id).unwrap();
        e.attributes.insert("admiralty_grade".into(), format!("{sg}{ig}"));
        e.attributes.insert("humint_reliability".into(), admiralty_label(sg, ig));
        let tag = format!("humint:{sg}{ig}");
        if !e.tags.iter().any(|t| *t == tag) { e.tags.push(tag); }
        n += 1;
    }
    n
}
fn admiralty_label(sg: char, ig: char) -> String {
    let s = match sg { 'A' => "completely reliable", 'B' => "usually reliable", 'C' => "fairly reliable", 'D' => "not usually reliable", 'E' => "unreliable", _ => "reliability unknown" };
    let i = match ig { '1' => "confirmed", '2' => "probably true", '3' => "possibly true", '4' => "doubtful", '5' => "improbable", _ => "cannot be judged" };
    format!("{s}; {i}")
}

// ---------------------------------------------------------------------------
// SIGINT — communication-pattern correlation (metadata only)
// ---------------------------------------------------------------------------
fn sigint_comms(graph: &mut KnowledgeGraph) -> usize {
    // For each Communication, collect its connected Account/Person endpoints; any
    // two endpoints on the same comm are `co_communicated_with`.
    let comm_ids: Vec<String> = graph.entities.iter().filter(|(_, e)| e.kind == EntityKind::Communication).map(|(id, _)| id.clone()).collect();
    if comm_ids.is_empty() {
        return 0;
    }
    let mut endpoints: HashMap<String, Vec<String>> = HashMap::new();
    for r in &graph.relationships {
        for (comm, other) in [(&r.source_id, &r.target_id), (&r.target_id, &r.source_id)] {
            if comm_ids.contains(comm) {
                if let Some(o) = graph.entities.get(other) {
                    if matches!(o.kind, EntityKind::Account | EntityKind::Person | EntityKind::Suspect | EntityKind::Victim) {
                        endpoints.entry(comm.clone()).or_default().push(other.clone());
                    }
                }
            }
        }
    }
    let mut edges = Vec::new();
    for (_comm, eps) in &endpoints {
        let mut u: Vec<&String> = eps.iter().collect();
        u.sort(); u.dedup();
        if u.len() > 8 { continue; } // group chat / broadcast — skip the clique
        for i in 0..u.len() {
            for j in (i + 1)..u.len() {
                let mut r = Relationship::new(u[i].clone(), "co_communicated_with", u[j].clone(), 0.55);
                r.source_reference = Some("sigint:shared-comm".into());
                edges.push(r);
            }
        }
    }
    let mut n = 0;
    for e in edges {
        let before = graph.relationship_count();
        graph.add_relationship(e);
        if graph.relationship_count() > before { n += 1; }
    }
    n
}

// ---------------------------------------------------------------------------
// OSINT — selector/handle reuse across sources
// ---------------------------------------------------------------------------
fn osint_selectors(graph: &mut KnowledgeGraph) -> usize {
    // Accounts whose handle (label minus any @domain) matches → same-actor
    // candidate across platforms/sources.
    let mut by_handle: HashMap<String, Vec<String>> = HashMap::new();
    for (id, e) in &graph.entities {
        if e.kind == EntityKind::Account {
            let handle = e.label.split('@').next().unwrap_or("").trim().to_lowercase();
            if handle.len() >= 4 {
                by_handle.entry(handle).or_default().push(id.clone());
            }
        }
    }
    let mut edges = Vec::new();
    for (_h, ids) in &by_handle {
        if ids.len() < 2 || ids.len() > 6 { continue; }
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let mut r = Relationship::new(ids[i].clone(), "same_selector_as", ids[j].clone(), 0.5);
                r.source_reference = Some("osint:handle-reuse".into());
                edges.push(r);
            }
        }
    }
    let mut n = 0;
    for e in edges {
        let before = graph.relationship_count();
        graph.add_relationship(e);
        if graph.relationship_count() > before { n += 1; }
    }
    n
}
