//! Threshold calibration: measure how the anomaly, perceptual-hash and link-
//! prediction thresholds behave on THIS dataset, and recommend values. There is
//! no universal threshold — the right cut depends on the volume and distribution
//! of the data, so this reports the real spread and picks a sensible operating
//! point (precision-first: keep flag rates low enough to be actionable).
//!
//! Triggered by `CORTEX_CALIBRATE=1` on a run; prints after the pipeline. Reads
//! the finished graph, so all derived attributes/metrics are already present.

use crate::ontology::KnowledgeGraph;
use owo_colors::OwoColorize;

pub fn report(graph: &KnowledgeGraph) {
    let n = graph.entity_count();
    println!("\n{}", "── Threshold calibration ──".bright_white().bold());
    println!("  dataset: {} entities · {} relationships\n", n, graph.relationship_count());

    anomaly_section(graph, n);
    linkpred_section(graph);
    phash_section(graph);

    println!("\n  {}", "Set the recommended env vars, then re-run. Re-calibrate whenever the data volume/shape changes materially.".dimmed());
}

fn pct(part: usize, whole: usize) -> f32 {
    if whole == 0 { 0.0 } else { part as f32 / whole as f32 * 100.0 }
}

fn anomaly_section(graph: &KnowledgeGraph, n: usize) {
    println!("  {}", "Anomaly (peer-relative outliers)".cyan());
    let zs = [2.5f32, 3.0, 3.5, 4.0];
    let mut counts = Vec::new();
    for z in zs {
        let c = crate::anomaly::detect_with(graph, z).anomalies.len();
        counts.push(c);
        println!("    z ≥ {:<4}  → {:>3} flagged ({:.1}%)", z, c, pct(c, n));
    }
    // Recommend the smallest z whose flag rate is <= ~2% (actionable), else the
    // strictest. This favours precision.
    let target = (n as f32 * 0.02).ceil() as usize;
    let mut rec = *zs.last().unwrap();
    for (i, z) in zs.iter().enumerate() {
        if counts[i] <= target.max(1) { rec = *z; break; }
    }
    println!("    → recommend threshold z ≈ {} (default risk weight 0.22; flags stay ~≤2%)\n", rec.to_string().green());
}

fn linkpred_section(graph: &KnowledgeGraph) {
    println!("  {}", "Link prediction (likely-but-absent edges)".cyan());
    for ms in [2usize, 3, 4] {
        let preds = crate::linkpred::predict_with(graph, ms, usize::MAX);
        let scores: Vec<f32> = preds.iter().map(|p| p.score).collect();
        let (min, med, max) = triples(&scores);
        println!("    min shared ≥ {}  → {:>3} candidates · Adamic-Adar min/med/max = {:.2}/{:.2}/{:.2}", ms, preds.len(), min, med, max);
    }
    let base = crate::linkpred::predict_with(graph, 2, usize::MAX).len();
    // Recommend top_k around the count at min-shared 2, capped so the graph view
    // stays readable.
    let rec_k = base.clamp(0, 25);
    println!("    → recommend min shared = {}, top_k ≈ {} (dashed 'predicted' edges)\n", "2".green(), rec_k.to_string().green());
}

fn phash_section(graph: &KnowledgeGraph) {
    let refs = crate::references::load();
    let phashes: Vec<String> = graph.entities.values()
        .filter_map(|e| e.attributes.get("perceptual_hash").cloned())
        .collect();
    println!("  {}", "Perceptual hash (near-duplicate distance)".cyan());
    if refs.is_empty() {
        println!("    (no reference sources loaded — set CORTEX_REFS to calibrate this)\n");
        return;
    }
    if phashes.is_empty() {
        println!("    (no perceptual hashes in the data)\n");
        return;
    }
    let mut dists: Vec<u32> = phashes.iter().filter_map(|p| refs.nearest_perceptual(p)).collect();
    if dists.is_empty() {
        println!("    (no comparable perceptual references — check bit-lengths)\n");
        return;
    }
    dists.sort_unstable();
    for d in [4u32, 6, 8, 10, 12, 16] {
        let c = dists.iter().filter(|&&x| x <= d).count();
        println!("    ≤ {:<2} bits → {:>3} match(es) ({:.1}% of {} hashes)", d, c, pct(c, phashes.len()), phashes.len());
    }
    // Recommend the distance just below the first big jump (a natural gap between
    // near-duplicates and unrelated images), else a conservative 10.
    let rec = recommend_gap(&dists).unwrap_or(10);
    println!("    → recommend CORTEX_PHASH_MAXDIST = {} (nearest-distance distribution suggests the cut)\n", rec.to_string().green());
}

/// min / median / max of a slice.
fn triples(v: &[f32]) -> (f32, f32, f32) {
    if v.is_empty() { return (0.0, 0.0, 0.0); }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (s[0], s[s.len() / 2], s[s.len() - 1])
}

/// Find a natural gap in the sorted distance distribution: the cut just below the
/// largest jump between consecutive distances (separates near-dups from the rest).
fn recommend_gap(sorted: &[u32]) -> Option<u32> {
    if sorted.len() < 2 { return sorted.first().map(|d| d + 2); }
    let mut best_gap = 0u32;
    let mut cut = sorted[0];
    for w in sorted.windows(2) {
        let gap = w[1].saturating_sub(w[0]);
        if gap > best_gap { best_gap = gap; cut = w[0]; }
    }
    // Only trust the gap if it's meaningful (>=4 bits); else conservative default.
    if best_gap >= 4 { Some(cut) } else { None }
}
