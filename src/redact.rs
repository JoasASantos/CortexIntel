//! PII redaction for exports (governance). Produces a redacted copy of the
//! consolidated case so a brief/bundle can be shared without leaking personal
//! data: emails, phones, CPF/SSN-like ids and person/victim labels are masked,
//! while structure, kinds, risk and relationships are preserved. Deterministic;
//! the same input always masks to the same token so cross-references still line up.

use serde_json::Value;

/// Mask a raw string value: redact embedded emails/phones/national-ids, and if
/// the whole value looks like a personal name, replace it with a stable token.
pub fn mask_value(s: &str) -> String {
    let mut out = redact_patterns(s);
    // Whole-value personal-name heuristic (2+ capitalized words, no digits/@).
    if looks_like_name(&out) {
        out = format!("PERSON-{}", short_token(s));
    }
    out
}

/// Redact PII patterns inside a string, keeping the rest intact.
pub fn redact_patterns(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for tok in split_keep(s) {
        if is_email(&tok) {
            out.push_str(&format!("[email:{}]", short_token(&tok)));
        } else if is_phone(&tok) {
            out.push_str("[phone]");
        } else if is_national_id(&tok) {
            out.push_str("[id]");
        } else {
            out.push_str(&tok);
        }
    }
    out
}

/// Return a redacted copy of the consolidated case JSON.
pub fn redact_case(case: &Value) -> Value {
    let mut c = case.clone();
    if let Some(groups) = c.get_mut("entities").and_then(|e| e.as_object_mut()) {
        for (kind, arr) in groups.iter_mut() {
            let personal = matches!(kind.as_str(), "persons" | "victims" | "suspects");
            if let Some(list) = arr.as_array_mut() {
                for e in list.iter_mut() {
                    redact_entity(e, personal);
                }
            }
        }
    }
    c
}

fn redact_entity(e: &mut Value, personal_kind: bool) {
    if let Some(label) = e.get("label").and_then(|v| v.as_str()) {
        let masked = if personal_kind && looks_like_name(label) {
            format!("PERSON-{}", short_token(label))
        } else {
            mask_value(label)
        };
        e["label"] = Value::String(masked);
    }
    // Redact free-text/PII-ish attribute values (keep keys + structural ones).
    if let Some(attrs) = e.get_mut("attributes").and_then(|a| a.as_object_mut()) {
        const KEEP: &[&str] = &["latitude", "longitude", "lat", "lon", "risk_score", "risk_band",
            "betweenness", "pagerank", "community", "activity_hour", "ip_scope", "registrable_domain",
            "hash_type", "ref_match", "ref_severity", "total_value", "admiralty_grade"];
        for (k, v) in attrs.iter_mut() {
            if KEEP.contains(&k.as_str()) {
                continue;
            }
            if let Some(s) = v.as_str() {
                *v = Value::String(mask_value(s));
            }
        }
    }
}

fn split_keep(s: &str) -> Vec<String> {
    // Split on whitespace but keep the separators so text reads normally.
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
            out.push(ch.to_string());
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

fn is_email(t: &str) -> bool {
    let t = t.trim_matches(|c: char| !c.is_alphanumeric());
    t.matches('@').count() == 1 && t.split('@').nth(1).map(|d| d.contains('.')).unwrap_or(false)
}
fn is_phone(t: &str) -> bool {
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    let ok = t.chars().all(|c| c.is_ascii_digit() || "+-() ".contains(c));
    ok && (10..=15).contains(&digits)
}
fn is_national_id(t: &str) -> bool {
    // CPF (11 digits, often ###.###.###-##) or SSN (9, ###-##-####).
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    let ok = t.chars().all(|c| c.is_ascii_digit() || ".-".contains(c));
    ok && (digits == 9 || digits == 11) && t.chars().any(|c| ".-".contains(c))
}
fn looks_like_name(s: &str) -> bool {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() < 2 || words.len() > 4 { return false; }
    if s.contains('@') || s.chars().any(|c| c.is_ascii_digit()) { return false; }
    words.iter().all(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() >= 2)
}
/// Stable short token so the same value always maps to the same mask.
fn short_token(s: &str) -> String {
    let mut h: u64 = 1469598103934665603;
    for b in s.trim().to_lowercase().as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{:06x}", h & 0xffffff)
}
