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

/// A ranked next-best-action: the collection/verification that most reduces the
/// investigation's uncertainty, with the reason and estimated payoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub action: String,
    pub why: String,
    /// Estimated uncertainty reduction if done [0..1].
    pub uncertainty_reduction: f32,
    /// Estimated effort [0..1] (higher = costlier).
    pub effort: f32,
    /// Composite priority = value / effort, normalized [0..1].
    pub priority: f32,
    /// Where to do it (a GUI destination hint).
    pub target: String,
    /// Entity ids this action concerns (for GUI focus), if any.
    pub entity_ids: Vec<String>,
}

/// Rank the next-best-actions by how much they cut uncertainty per unit effort.
/// Deterministic: derived from real data-quality/structure gaps, not opinion.
pub fn next_best_actions(g: &KnowledgeGraph, risk: &RiskReport, domain: Domain) -> Vec<NextAction> {
    let l = lens(domain);
    let deg = degrees(g);
    let n = g.entity_count().max(1);
    let mut acts: Vec<NextAction> = Vec::new();

    let mut mk = |action: String, why: String, red: f32, effort: f32, target: &str, ids: Vec<String>| {
        let effort = effort.max(0.05);
        acts.push(NextAction { action, why, uncertainty_reduction: red, effort, priority: (red / effort).min(3.0) / 3.0, target: target.into(), entity_ids: ids });
    };

    // 1) Entities with no source — provenance gaps cap trust.
    let no_src: Vec<&crate::ontology::Entity> = g.entities.values().filter(|e| e.sources.is_empty()).collect();
    if !no_src.is_empty() {
        let frac = no_src.len() as f32 / n as f32;
        mk(
            format!("Trace the source of {} entities lacking provenance", no_src.len()),
            "Unsourced entities can't be trusted; establishing provenance directly raises confidence in every judgment that depends on them.".into(),
            (0.35 + frac * 0.4).min(0.8), 0.4, "entities",
            no_src.iter().take(20).map(|e| e.id.clone()).collect(),
        );
    }

    // 2) Ambiguous shared hub — resolving benign-vs-real is high leverage.
    let hub_kinds = [EntityKind::Ip, EntityKind::Device, EntityKind::Wallet, EntityKind::Domain];
    if let Some((id, d)) = g.entities.iter()
        .filter(|(_, e)| hub_kinds.contains(&e.kind))
        .map(|(id, _)| (id, *deg.get(id).unwrap_or(&0)))
        .filter(|(_, d)| *d >= 3)
        .max_by_key(|(_, d)| *d) {
        let e = &g.entities[id];
        mk(
            format!("Verify whether the shared {} \"{}\" is a real link or a benign aggregator", e.kind.as_str(), e.label),
            format!("{} entities hinge on this hub — {}. Confirming it collapses or confirms the whole cluster's meaning.", d, l.hub_meaning),
            0.6, 0.3, "graph", vec![id.clone()],
        );
    }

    // 3) Isolated entities — correlation is incomplete.
    let isolated = g.entities.keys().filter(|id| *deg.get(*id).unwrap_or(&0) == 0).count();
    if isolated > 0 {
        mk(
            format!("Enrich {} isolated entities to reveal relationships", isolated),
            "Entities with no known relations mean the picture is under-connected; enriching them can surface links that change clusters and risk.".into(),
            (0.25 + (isolated as f32 / n as f32) * 0.35).min(0.7), 0.55, "sources", vec![],
        );
    }

    // 4) Unconfirmed AI hypotheses — verify before relying on them.
    let hyp: Vec<&crate::ontology::Entity> = g.entities.values().filter(|e| e.tags.iter().any(|t| t == "hypothesis")).collect();
    if !hyp.is_empty() {
        mk(
            format!("Confirm or reject {} AI-proposed entities", hyp.len()),
            "AI hypotheses are inference, not evidence; validating them removes the biggest source of speculative risk in the graph.".into(),
            0.5, 0.35, "entities", hyp.iter().take(20).map(|e| e.id.clone()).collect(),
        );
    }

    // 5) If the case is high/critical, the single highest-value move is verifying the top actor.
    if let Some(top) = risk.assessments.iter().filter(|a| a.risk_score >= 0.6).max_by(|a, b| a.risk_score.partial_cmp(&b.risk_score).unwrap()) {
        mk(
            format!("Verify the highest-risk {} \"{}\" ({:.2})", l.actor, top.entity_label, top.risk_score),
            "The top-risk entity drives the case score; confirming or clearing it moves the decision the most.".into(),
            0.55, 0.3, "graph", vec![top.entity_id.clone()],
        );
    }

    acts.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
    acts.truncate(6);
    acts
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

/// Render the ranked next-best-actions as a Markdown section.
pub fn nba_to_markdown(items: &[NextAction]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut s = String::from("## Next best actions\n\n");
    s.push_str("_Ranked by how much each step reduces uncertainty per unit of effort._\n\n");
    for (i, a) in items.iter().enumerate() {
        s.push_str(&format!(
            "**{}. {}**  \n_Uncertainty ↓ {:.0}% · effort {:.0}% · priority {:.0}%_  \n{}\n\n",
            i + 1, a.action, a.uncertainty_reduction * 100.0, a.effort * 100.0, a.priority * 100.0, a.why
        ));
    }
    s
}
