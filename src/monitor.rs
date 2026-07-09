//! Continuous intelligence: the "living picture". When a project is re-run, this
//! computes what CHANGED since the previous result (new/removed entities, risk
//! that rose, new relationships) and evaluates standing watchlist rules that fire
//! alerts. Deterministic; operates on the consolidated case JSON.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// Pull the flat list of entities (across the kind-grouped buckets) as
/// (label, kind, risk) from a consolidated case document.
fn entities(case: &Value) -> Vec<(String, String, f64)> {
    let mut out = Vec::new();
    if let Some(groups) = case.get("entities").and_then(|e| e.as_object()) {
        for arr in groups.values() {
            if let Some(a) = arr.as_array() {
                for e in a {
                    let label = e.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let kind = e.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let risk = e.get("risk_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    if !label.is_empty() {
                        out.push((label, kind, risk));
                    }
                }
            }
        }
    }
    out
}

/// Compute the change report between a previous and a current case.
pub fn diff(prev: &Value, curr: &Value) -> Value {
    let pe = entities(prev);
    let ce = entities(curr);
    let prev_map: HashMap<&str, f64> = pe.iter().map(|(l, _, r)| (l.as_str(), *r)).collect();
    let prev_set: HashSet<&str> = pe.iter().map(|(l, _, _)| l.as_str()).collect();
    let curr_set: HashSet<&str> = ce.iter().map(|(l, _, _)| l.as_str()).collect();

    let mut new_entities = Vec::new();
    let mut risk_up = Vec::new();
    for (label, kind, risk) in &ce {
        if !prev_set.contains(label.as_str()) {
            new_entities.push(json!({"label": label, "kind": kind, "risk": risk}));
        } else if let Some(&pr) = prev_map.get(label.as_str()) {
            if *risk - pr >= 0.15 {
                risk_up.push(json!({"label": label, "kind": kind, "from": pr, "to": risk}));
            }
        }
    }
    let removed: Vec<Value> = pe.iter().filter(|(l, _, _)| !curr_set.contains(l.as_str()))
        .map(|(l, k, _)| json!({"label": l, "kind": k})).collect();

    let prev_rel = prev.get("relationships").and_then(|r| r.as_array()).map(|a| a.len()).unwrap_or(0);
    let curr_rel = curr.get("relationships").and_then(|r| r.as_array()).map(|a| a.len()).unwrap_or(0);

    // Sort the highlights so the most severe are first.
    new_entities.sort_by(|a, b| b["risk"].as_f64().partial_cmp(&a["risk"].as_f64()).unwrap());
    risk_up.sort_by(|a, b| b["to"].as_f64().partial_cmp(&a["to"].as_f64()).unwrap());

    json!({
        "new_entities": new_entities,
        "removed_entities": removed,
        "risk_increased": risk_up,
        "relationships_delta": curr_rel as i64 - prev_rel as i64,
        "summary": summarize(&new_entities, &risk_up, &removed, curr_rel as i64 - prev_rel as i64),
    })
}

fn summarize(new_e: &[Value], risk_up: &[Value], removed: &[Value], rel_delta: i64) -> String {
    let mut parts = Vec::new();
    if !new_e.is_empty() { parts.push(format!("{} new entit{}", new_e.len(), if new_e.len() == 1 { "y" } else { "ies" })); }
    if !risk_up.is_empty() { parts.push(format!("{} risk increase(s)", risk_up.len())); }
    if !removed.is_empty() { parts.push(format!("{} removed", removed.len())); }
    if rel_delta != 0 { parts.push(format!("{}{} relationship(s)", if rel_delta > 0 { "+" } else { "" }, rel_delta)); }
    if parts.is_empty() { "No material change since the last run.".into() } else { format!("Since last run: {}.", parts.join(", ")) }
}

/// A standing watchlist rule. Fires when a matching entity is present.
/// Rule fields: kind (optional), min_risk (optional), label_contains (optional).
pub fn evaluate_watchlist(case: &Value, rules: &Value) -> Vec<Value> {
    let mut alerts = Vec::new();
    let Some(rule_arr) = rules.as_array() else { return alerts };
    let ents = entities(case);
    for rule in rule_arr {
        let want_kind = rule.get("kind").and_then(|v| v.as_str());
        let min_risk = rule.get("min_risk").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let contains = rule.get("label_contains").and_then(|v| v.as_str()).map(|s| s.to_lowercase());
        let name = rule.get("name").and_then(|v| v.as_str()).unwrap_or("watchlist rule");
        let mut hits = Vec::new();
        for (label, kind, risk) in &ents {
            if let Some(k) = want_kind { if k != kind { continue; } }
            if *risk < min_risk { continue; }
            if let Some(c) = &contains { if !label.to_lowercase().contains(c) { continue; } }
            hits.push(json!({"label": label, "kind": kind, "risk": risk}));
        }
        if !hits.is_empty() {
            hits.sort_by(|a, b| b["risk"].as_f64().partial_cmp(&a["risk"].as_f64()).unwrap());
            alerts.push(json!({"rule": name, "count": hits.len(), "hits": hits.into_iter().take(10).collect::<Vec<_>>()}));
        }
    }
    alerts
}
