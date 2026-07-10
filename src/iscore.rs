//! Intelligence Score: one explainable 0–100 index per entity that blends the
//! signals the engine already computed — risk, brokerage (betweenness),
//! importance (PageRank), anomaly, connectivity and reference/known-hash matches
//! — so a non-technical user gets a single meaningful number *with the reasons*.
//! Deterministic; written onto each entity as `intel_score` + `intel_breakdown`.

use crate::ontology::KnowledgeGraph;

const W_RISK: f32 = 0.40;
const W_BROKER: f32 = 0.18;
const W_PAGERANK: f32 = 0.10;
const W_ANOMALY: f32 = 0.14;
const W_CONNECT: f32 = 0.08;
const W_REFHIT: f32 = 0.10;

pub fn compute(graph: &mut KnowledgeGraph) -> usize {
    let degree = graph.degree_centrality();
    let max_deg = degree.values().copied().max().unwrap_or(1).max(1) as f32;
    let ids: Vec<String> = graph.entities.keys().cloned().collect();
    let mut n = 0;
    for id in ids {
        let deg = *degree.get(&id).unwrap_or(&0) as f32 / max_deg;
        let e = graph.entities.get_mut(&id).unwrap();
        let g = |k: &str| e.attributes.get(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        let risk = e.risk_score.unwrap_or(0.0);
        let broker = g("betweenness");
        let pr = g("pagerank");
        let anomaly = g("anomaly_score");
        let refhit = if e.attributes.contains_key("ref_source") { 1.0 } else { 0.0 };

        let score = (W_RISK * risk + W_BROKER * broker + W_PAGERANK * pr
            + W_ANOMALY * anomaly + W_CONNECT * deg + W_REFHIT * refhit)
            .clamp(0.0, 1.0);
        let pct = (score * 100.0).round() as i32;

        // Breakdown: the top contributing factors, in points, for the "why".
        let mut parts: Vec<(String, f32)> = vec![
            ("risk".into(), W_RISK * risk * 100.0),
            ("broker".into(), W_BROKER * broker * 100.0),
            ("importance".into(), W_PAGERANK * pr * 100.0),
            ("anomaly".into(), W_ANOMALY * anomaly * 100.0),
            ("connectivity".into(), W_CONNECT * deg * 100.0),
            ("known-match".into(), W_REFHIT * refhit * 100.0),
        ];
        parts.retain(|(_, v)| *v >= 0.5);
        parts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let breakdown = parts.iter().take(4).map(|(k, v)| format!("{k} +{:.0}", v)).collect::<Vec<_>>().join(", ");

        e.attributes.insert("intel_score".into(), pct.to_string());
        if !breakdown.is_empty() {
            e.attributes.insert("intel_breakdown".into(), breakdown);
        }
        n += 1;
    }
    n
}
