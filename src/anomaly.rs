//! Anomaly detection: flag entities that deviate from their PEERS (same kind).
//! "This account's degree is far above other accounts", "this node bridges the
//! graph unlike its peers", "activity at 3am". Deterministic and offline, using
//! a robust outlier test (median + MAD, with a ratio fallback for small groups)
//! plus a simple behavioural rule. Runs after network science (so betweenness is
//! available) and before risk, writing an `anomaly_score` that risk picks up.

use crate::ontology::{EntityKind, KnowledgeGraph};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Anomaly {
    pub entity_id: String,
    /// 0..1 strength of the anomaly.
    pub score: f32,
    /// Human-readable, explainable reason(s).
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct AnomalyReport {
    pub anomalies: Vec<Anomaly>,
}

/// Numeric features compared within each entity-kind peer group.
const FEATURES: &[&str] = &["degree", "betweenness", "pagerank", "sources"];

pub fn detect(graph: &KnowledgeGraph) -> AnomalyReport {
    let degree = graph.degree_centrality();
    let mut report = AnomalyReport::default();

    // Collect per-entity feature values.
    struct Row<'a> {
        id: &'a str,
        kind: EntityKind,
        feats: HashMap<&'static str, f32>,
        activity_hour: Option<u8>,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (id, e) in &graph.entities {
        let mut feats = HashMap::new();
        feats.insert("degree", *degree.get(id).unwrap_or(&0) as f32);
        feats.insert("betweenness", e.attributes.get("betweenness").and_then(|v| v.parse().ok()).unwrap_or(0.0));
        feats.insert("pagerank", e.attributes.get("pagerank").and_then(|v| v.parse().ok()).unwrap_or(0.0));
        feats.insert("sources", e.sources.len() as f32);
        let activity_hour = e.attributes.get("activity_hour").and_then(|v| v.parse::<u8>().ok());
        rows.push(Row { id, kind: e.kind, feats, activity_hour });
    }

    // Per (kind, feature) distribution → robust outlier test.
    let mut by_kind: HashMap<EntityKind, Vec<usize>> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        by_kind.entry(r.kind).or_default().push(i);
    }

    // entity id -> (best score, reasons)
    let mut hits: HashMap<String, (f32, Vec<String>)> = HashMap::new();
    for (_kind, idxs) in &by_kind {
        if idxs.len() < 4 {
            continue; // too few peers to judge "typical"
        }
        for feat in FEATURES {
            let vals: Vec<f32> = idxs.iter().map(|&i| rows[i].feats[feat]).collect();
            let (median, mad) = median_mad(&vals);
            for &i in idxs {
                let x = rows[i].feats[feat];
                if x <= median {
                    continue; // only flag high-side outliers
                }
                let z = if mad > 1e-6 { (x - median) / mad } else { 0.0 };
                let ratio = if median > 1e-6 { x / median } else { f32::INFINITY };
                // Robust z for spread groups; ratio fallback when MAD collapses
                // (e.g. most peers identical) — needs a real gap, not just +1.
                let outlier = (mad > 1e-6 && z >= 3.5) || (mad <= 1e-6 && ratio >= 2.0 && (x - median) >= 2.0);
                if outlier {
                    let strength = if z >= 6.0 || ratio >= 4.0 { 0.8 } else { 0.62 };
                    let reason = if mad > 1e-6 {
                        format!("{feat} {:.2} is {:.1}σ above peers", x, z)
                    } else {
                        format!("{feat} {:.0} is {:.1}× the peer median", x, ratio)
                    };
                    let entry = hits.entry(rows[i].id.to_string()).or_insert((0.0, Vec::new()));
                    if strength > entry.0 {
                        entry.0 = strength;
                    }
                    entry.1.push(reason);
                }
            }
        }
    }

    // Behavioural rule: activity in the small hours (independent of peer group).
    for r in &rows {
        if let Some(h) = r.activity_hour {
            if h <= 5 {
                let entry = hits.entry(r.id.to_string()).or_insert((0.0, Vec::new()));
                entry.0 = entry.0.max(0.5);
                entry.1.push(format!("activity at {:02}:00 (off-hours)", h));
            }
        }
    }

    for (id, (score, reasons)) in hits {
        report.anomalies.push(Anomaly { entity_id: id, score, reason: reasons.join("; ") });
    }
    report.anomalies.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    report
}

/// Median and MAD (median absolute deviation, scaled to be a σ estimate).
fn median_mad(values: &[f32]) -> (f32, f32) {
    let med = median(values);
    let devs: Vec<f32> = values.iter().map(|v| (v - med).abs()).collect();
    let mad = median(&devs) * 1.4826;
    (med, mad)
}

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}
