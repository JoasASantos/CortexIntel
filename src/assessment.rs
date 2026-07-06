//! The information → intelligence step: turn the graph + risk into natural-language
//! ASSESSMENTS of the form "{observation} because {evidence}; confidence {x};
//! action: {next step}". Fully deterministic (offline), each statement links back
//! to the entities/relationships that support it. The vertical LENS only changes
//! vocabulary and emphasis — same engine, sharper output per domain.

use crate::config::Domain;
use crate::ontology::{EntityKind, KnowledgeGraph};
use crate::risk::RiskReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// Natural-language statement (already lens-flavored for the vertical).
    pub statement: String,
    /// Calibrated confidence [0..1].
    pub confidence: f32,
    /// Human evidence references (entity labels / relationship phrasings).
    pub evidence: Vec<String>,
    /// Entity ids the statement is anchored to (for the GUI to focus).
    pub evidence_ids: Vec<String>,
    /// Recommended next step.
    pub action: String,
    /// "observed" (from data) vs "inferred" (structural inference).
    pub basis: String,
}

/// Per-vertical vocabulary lens. Same structure, different words/emphasis.
struct Lens {
    /// noun for the whole picture ("threat picture", "customer picture"…)
    picture: &'static str,
    /// what a shared hub implies in this vertical
    hub_meaning: &'static str,
    /// what a high-risk actor is called
    actor: &'static str,
    /// verb for the recommended escalation
    escalate: &'static str,
}

fn lens(d: Domain) -> Lens {
    match d {
        Domain::Cybersecurity => Lens { picture: "threat picture", hub_meaning: "shared infrastructure often indicates coordinated activity or a common operator", actor: "threat actor", escalate: "hand to SOC/DFIR for containment" },
        Domain::Fraud | Domain::Kyc | Domain::Finance => Lens { picture: "fraud picture", hub_meaning: "accounts sharing a device/IP/wallet frequently signal a mule ring or single controller", actor: "high-exposure account", escalate: "open a financial investigation and freeze pending review" },
        Domain::ChildProtection => Lens { picture: "victim-protection picture", hub_meaning: "shared accounts/infrastructure can link a distribution network", actor: "at-risk or suspect entity", escalate: "escalate to child-protection and preserve evidence" },
        Domain::Commerce | Domain::Education => Lens { picture: "customer picture", hub_meaning: "accounts clustering on a shared attribute may be one household or a coordinated segment", actor: "priority account", escalate: "route to the owning team for outreach or review" },
        Domain::Logistics => Lens { picture: "operations picture", hub_meaning: "assets converging on a node may be a bottleneck or single point of failure", actor: "critical asset", escalate: "flag for operational review" },
        Domain::Military => Lens { picture: "situational picture", hub_meaning: "entities converging on a node may indicate coordination or a key facilitator", actor: "entity of interest", escalate: "route to human analyst review — never automated action" },
        _ => Lens { picture: "intelligence picture", hub_meaning: "entities sharing an attribute are correlated and worth examining together", actor: "priority entity", escalate: "route for human review" },
    }
}

fn degrees(g: &KnowledgeGraph) -> HashMap<String, usize> {
    g.degree_centrality()
}

/// Build the assessment for a run. Deterministic; ordered by confidence.
pub fn assess(g: &KnowledgeGraph, risk: &RiskReport, domain: Domain) -> Vec<Assessment> {
    let l = lens(domain);
    let deg = degrees(g);
    let mut out: Vec<Assessment> = Vec::new();

    // 1) Overall posture from case risk.
    let band = &risk.case_risk_band;
    let (conf, verb) = match band.as_str() {
        "critical" => (0.8, "demands immediate attention"),
        "high" => (0.7, "warrants prioritized review"),
        "medium" => (0.5, "shows moderate signals"),
        _ => (0.4, "appears low-signal"),
    };
    out.push(Assessment {
        statement: format!("The {} {} — overall case risk is {} ({:.2}).", l.picture, verb, band, risk.case_risk_score),
        confidence: conf,
        evidence: vec![format!("{} entities, {} relationships", g.entity_count(), g.relationship_count())],
        evidence_ids: vec![],
        action: format!("Review the top prioritized entities; {} if confirmed.", l.escalate),
        basis: "observed".into(),
    });

    // 2) Shared-hub coordination (structural inference).
    let hub_kinds = [EntityKind::Ip, EntityKind::Device, EntityKind::Wallet, EntityKind::Domain, EntityKind::Group];
    let mut hubs: Vec<(&String, usize, EntityKind)> = g
        .entities
        .iter()
        .filter(|(_, e)| hub_kinds.contains(&e.kind))
        .map(|(id, e)| (id, *deg.get(id).unwrap_or(&0), e.kind))
        .filter(|(_, d, _)| *d >= 3)
        .collect();
    hubs.sort_by(|a, b| b.1.cmp(&a.1));
    if let Some((id, d, kind)) = hubs.first() {
        let e = &g.entities[*id];
        let lk = (0.35 + *d as f32 * 0.05).min(0.9);
        out.push(Assessment {
            statement: format!("{} entities converge on the shared {} \"{}\" — {}.", d, kind.as_str(), e.label, l.hub_meaning),
            confidence: lk,
            evidence: vec![format!("{} connections to {}", d, e.label)],
            evidence_ids: vec![(*id).clone()],
            action: "Isolate this cluster and expand its members; confirm whether the shared hub is a genuine link or a benign aggregator.".into(),
            basis: "inferred".into(),
        });
    }

    // 3) Concentration of risk in few actors.
    let mut ranked: Vec<_> = risk.assessments.iter().filter(|a| a.risk_score >= 0.6).collect();
    ranked.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());
    if !ranked.is_empty() {
        let top = &ranked[..ranked.len().min(3)];
        let names: Vec<String> = top.iter().map(|a| a.entity_label.clone()).collect();
        out.push(Assessment {
            statement: format!("Risk concentrates in {} {}{}: {}.", top.len(), l.actor, if top.len() > 1 { "s" } else { "" }, names.join(", ")),
            confidence: (top[0].risk_score * 0.9).min(0.85),
            evidence: top.iter().map(|a| format!("{} — {} ({:.2})", a.entity_label, a.risk_band, a.risk_score)).collect(),
            evidence_ids: top.iter().map(|a| a.entity_id.clone()).collect(),
            action: format!("Verify the top {}{} first; {}.", l.actor, if top.len() > 1 { "s" } else { "" }, l.escalate),
            basis: "observed".into(),
        });
    }

    // 4) Duplicate/identity collision (data-quality caveat that bounds confidence).
    let mut by_key: HashMap<String, usize> = HashMap::new();
    for e in g.entities.values() {
        *by_key.entry(format!("{}|{}", e.kind.as_str(), e.label.to_lowercase())).or_insert(0) += 1;
    }
    let dups: usize = by_key.values().filter(|c| **c > 1).map(|c| *c).sum();
    if dups > 1 {
        out.push(Assessment {
            statement: format!("{} entities appear to be duplicates or conflated identities, which can distort clusters and inflate risk.", dups),
            confidence: 0.6,
            evidence: vec![format!("{} label collisions detected", dups)],
            evidence_ids: vec![],
            action: "Resolve duplicates before drawing firm conclusions — treat cluster sizes as upper bounds.".into(),
            basis: "observed".into(),
        });
    }

    out.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    out
}

/// Render assessments as a Markdown "Assessment" section.
pub fn to_markdown(items: &[Assessment]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut s = String::from("## Assessment\n\n");
    s.push_str("_Data → information → intelligence. Each judgment states its confidence and the evidence behind it. The AI supports decisions; it does not decide._\n\n");
    for (i, a) in items.iter().enumerate() {
        s.push_str(&format!(
            "**{}. {}**  \n_Confidence: {:.0}% · basis: {}_  \n",
            i + 1, a.statement, a.confidence * 100.0, a.basis
        ));
        if !a.evidence.is_empty() {
            s.push_str(&format!("Evidence: {}.  \n", a.evidence.join("; ")));
        }
        s.push_str(&format!("Action: {}\n\n", a.action));
    }
    s
}
