//! Risk prioritization. A transparent heuristic scorer (graph structure +
//! DATA.md risk features) produces a baseline; the LLM risk agent can raise or
//! annotate scores. Output mirrors DATA.md §24 (explainable AI assessment).

use crate::config::Domain;
use crate::ontology::{EntityKind, KnowledgeGraph, RiskBand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One explainable assessment for an entity (DATA.md §24).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    pub entity_id: String,
    pub entity_label: String,
    pub entity_kind: String,
    pub risk_score: f32,
    pub risk_band: String,
    pub top_factors: Vec<String>,
    pub recommended_action: String,
    pub requires_human_review: bool,
    pub explanation: String,
    pub source: String, // "heuristic" | "heuristic+llm"
}

/// Whole-run risk output.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskReport {
    pub case_risk_score: f32,
    pub case_risk_band: String,
    pub assessments: Vec<Assessment>,
}

/// High-signal attribute/tag tokens that raise risk, weighted. Mirrors the
/// DATA.md "Campos de decisão mais úteis" / risk features.
fn signal_weights(domain: Domain) -> Vec<(&'static str, f32)> {
    let mut base = vec![
        ("imminent", 0.9),
        ("imminent_risk", 0.9),
        ("child_imminent_risk", 0.95),
        ("rescue", 0.85),
        ("threat", 0.6),
        ("coercion", 0.6),
        ("sextortion", 0.7),
        ("grooming", 0.65),
        ("distribution", 0.55),
        ("production", 0.6),
        ("commercial", 0.5),
        ("critical", 0.8),
        ("high", 0.6),
        ("malware", 0.6),
        ("exploit", 0.6),
        ("fraud", 0.55),
        ("laundering", 0.6),
        ("repeat_offender", 0.7),
        ("multiple_victims", 0.75),
        ("active", 0.4),
        ("urgent", 0.7),
    ];
    // Vertical-specific emphasis.
    match domain {
        Domain::Cybersecurity => base.extend([("cve", 0.6), ("c2", 0.7), ("ransomware", 0.85)]),
        Domain::Fraud => base.extend([("chargeback", 0.5), ("mule", 0.6), ("aml", 0.6)]),
        Domain::Health => base.extend([("adverse", 0.6), ("outbreak", 0.75)]),
        Domain::Logistics => base.extend([("delay", 0.4), ("disruption", 0.55)]),
        _ => {}
    }
    base
}

/// Kinds that inherently deserve attention.
fn kind_base(kind: EntityKind) -> f32 {
    match kind {
        EntityKind::Victim => 0.55,
        EntityKind::Suspect => 0.5,
        EntityKind::Malware | EntityKind::Vulnerability => 0.45,
        EntityKind::Incident => 0.4,
        EntityKind::Payment | EntityKind::Wallet => 0.35,
        EntityKind::Media | EntityKind::Evidence => 0.3,
        _ => 0.15,
    }
}

/// Compute baseline heuristic risk over the whole graph. `extra_signals` are
/// plugin-provided (token, weight) pairs merged into the built-in signal set.
pub fn score_graph(graph: &KnowledgeGraph, domain: Domain, extra_signals: &[(String, f32)]) -> RiskReport {
    let mut weights = signal_weights(domain);
    for (tok, w) in extra_signals {
        weights.push((Box::leak(tok.clone().into_boxed_str()), *w));
    }
    let degree = graph.degree_centrality();
    let max_deg = degree.values().copied().max().unwrap_or(1).max(1) as f32;
    // Reward engine: bounded per-key adjustments learned from analyst feedback.
    let reward_adj = crate::reward::adjustments();

    let mut assessments = Vec::new();
    for (id, e) in &graph.entities {
        let mut score = kind_base(e.kind);
        let mut factors: Vec<String> = Vec::new();

        // Signal tokens in attributes + tags.
        let hay = format!(
            "{} {} {}",
            e.tags.join(" ").to_lowercase(),
            e.attributes
                .iter()
                .map(|(k, v)| format!("{k} {v}"))
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase(),
            e.label.to_lowercase()
        );
        for (tok, w) in &weights {
            if hay.contains(tok) {
                score = score.max(*w);
                factors.push(format!("signal:{tok}"));
            }
        }

        // Connectivity: hubs are more important.
        let d = *degree.get(id).unwrap_or(&0) as f32;
        let conn = (d / max_deg) * 0.3;
        if conn > 0.0 {
            score += conn;
            factors.push(format!("connectivity:{}", d as usize));
        }

        // Brokerage: betweenness centrality (from network science) — a bridge /
        // chokepoint is structurally critical even when its raw degree is modest,
        // so it adds signal beyond connectivity. Written onto the entity by netsci.
        if let Some(bet) = e.attributes.get("betweenness").and_then(|v| v.parse::<f32>().ok()) {
            if bet > 0.0 {
                score += bet * 0.22;
                factors.push(format!("brokerage:{:.2}", bet));
            }
        }

        // Anomaly: being an outlier among peers is itself a reason to look.
        if let Some(an) = e.attributes.get("anomaly_score").and_then(|v| v.parse::<f32>().ok()) {
            if an > 0.0 {
                score += an * 0.22;
                factors.push(format!("anomaly:{:.2}", an));
            }
        }

        // Reward: nudge by what the analyst has confirmed/rejected for this kind/tags.
        let radj = crate::reward::entity_adjustment(&reward_adj, e.kind.as_str(), &e.tags);
        if radj.abs() > 1e-4 {
            score += radj;
            factors.push(format!("feedback:{radj:+.2}"));
        }

        let score = score.clamp(0.0, 1.0);
        let band = RiskBand::from_score(score);
        let requires_review = e.sensitive || band >= RiskBand::High;
        let action = recommend_action(e.kind, band, domain);
        if factors.is_empty() {
            factors.push("baseline".into());
        }

        assessments.push(Assessment {
            entity_id: id.clone(),
            entity_label: e.label.clone(),
            entity_kind: e.kind.as_str().to_string(),
            risk_score: score,
            risk_band: band.as_str().to_string(),
            top_factors: factors,
            recommended_action: action,
            requires_human_review: requires_review,
            explanation: format!(
                "Heuristic score from kind={}, connectivity and {} matched signal(s).",
                e.kind.as_str(),
                0
            ),
            source: "heuristic".into(),
        });
    }

    // Risk propagation: a fraction of a node's risk flows to its neighbours (one
    // damped hop). Proximity to a high-risk entity raises attention — a standard
    // graph-diffusion signal layered on top of the per-node score.
    {
        use std::collections::HashMap;
        let idx: HashMap<&str, usize> = assessments.iter().enumerate().map(|(i, a)| (a.entity_id.as_str(), i)).collect();
        let base: Vec<f32> = assessments.iter().map(|a| a.risk_score).collect();
        let mut nbr_max = vec![0f32; assessments.len()];
        for r in &graph.relationships {
            if let (Some(&si), Some(&ti)) = (idx.get(r.source_id.as_str()), idx.get(r.target_id.as_str())) {
                nbr_max[si] = nbr_max[si].max(base[ti]);
                nbr_max[ti] = nbr_max[ti].max(base[si]);
            }
        }
        for (i, a) in assessments.iter_mut().enumerate() {
            let flow = nbr_max[i] * 0.18;
            if flow > 0.02 && a.risk_score < 1.0 {
                let ns = (a.risk_score + flow).min(1.0);
                if ns > a.risk_score + 0.02 {
                    a.risk_score = ns;
                    a.risk_band = RiskBand::from_score(ns).as_str().to_string();
                    a.top_factors.push(format!("propagated:{flow:.2}"));
                    if a.risk_band == "critical" || a.risk_band == "high" { a.requires_human_review = true; }
                }
            }
        }
    }

    assessments.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());

    // Case risk = blend of the top entities.
    let top: Vec<f32> = assessments.iter().take(5).map(|a| a.risk_score).collect();
    let case_score = if top.is_empty() {
        0.0
    } else {
        top.iter().sum::<f32>() / top.len() as f32
    };

    RiskReport {
        case_risk_score: case_score,
        case_risk_band: RiskBand::from_score(case_score).as_str().to_string(),
        assessments,
    }
}

fn recommend_action(kind: EntityKind, band: RiskBand, domain: Domain) -> String {
    if band < RiskBand::Medium {
        return "monitor".into();
    }
    match (domain, kind) {
        (Domain::ChildProtection, EntityKind::Victim) => "prioritize_victim_identification".into(),
        (Domain::ChildProtection, EntityKind::Media | EntityKind::Url) => "prioritize_takedown".into(),
        (Domain::ChildProtection, _) => "escalate_to_child_protection".into(),
        (Domain::Cybersecurity, EntityKind::Malware | EntityKind::Incident) => "contain_and_investigate".into(),
        (Domain::Fraud, EntityKind::Payment | EntityKind::Wallet | EntityKind::Account) => "open_financial_investigation".into(),
        (_, _) if band >= RiskBand::High => "human_review_required".into(),
        _ => "check_related_entities".into(),
    }
}

/// Apply LLM-provided assessments on top of the heuristic baseline: take the
/// max score, merge factors, and mark the source as combined.
pub fn merge_llm(report: &mut RiskReport, llm: &serde_json::Value) {
    if let Some(cs) = llm.get("case_risk_score").and_then(|v| v.as_f64()) {
        report.case_risk_score = report.case_risk_score.max(cs as f32);
        report.case_risk_band = RiskBand::from_score(report.case_risk_score).as_str().to_string();
    }
    let Some(arr) = llm.get("assessments").and_then(|v| v.as_array()) else {
        return;
    };
    let mut by_id: HashMap<String, usize> = HashMap::new();
    for (i, a) in report.assessments.iter().enumerate() {
        by_id.insert(a.entity_id.clone(), i);
    }
    for item in arr {
        let Some(eid) = item.get("entity_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(&idx) = by_id.get(eid) else { continue };
        let a = &mut report.assessments[idx];
        if let Some(s) = item.get("risk_score").and_then(|v| v.as_f64()) {
            a.risk_score = a.risk_score.max(s as f32);
            a.risk_band = RiskBand::from_score(a.risk_score).as_str().to_string();
        }
        if let Some(fs) = item.get("top_factors").and_then(|v| v.as_array()) {
            for f in fs {
                if let Some(s) = f.as_str() {
                    let tag = format!("llm:{s}");
                    if !a.top_factors.contains(&tag) {
                        a.top_factors.push(tag);
                    }
                }
            }
        }
        if let Some(act) = item.get("recommended_action").and_then(|v| v.as_str()) {
            a.recommended_action = act.to_string();
        }
        if let Some(exp) = item.get("explanation").and_then(|v| v.as_str()) {
            a.explanation = exp.to_string();
        }
        if let Some(r) = item.get("requires_human_review").and_then(|v| v.as_bool()) {
            a.requires_human_review = a.requires_human_review || r;
        }
        a.source = "heuristic+llm".into();
    }
    report
        .assessments
        .sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());
}
