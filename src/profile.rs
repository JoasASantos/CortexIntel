//! Auto-ontology: profile the columns of an undeclared feed and infer, per
//! column, a semantic type and the ontology entity it maps to — so a user can
//! drop any CSV/JSON with no schema and still get a graph. Deterministic and
//! offline; the LLM layer may *propose* a mapping but it is always validated
//! against the real data (a column that doesn't exist yields nothing).

use crate::extract;
use crate::ontology::EntityKind;
use crate::sources::RecordBatch;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Inferred semantic type of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    Email,
    Phone,
    Ip,
    Url,
    Domain,
    Hash,
    Wallet,
    Money,
    Date,
    Id,
    PersonName,
    Boolean,
    Categorical,
    Number,
    FreeText,
    Location,
    Organization,
    NationalId,
    Empty,
}

impl SemanticType {
    /// The ontology entity a column of this type materializes (None = attribute only).
    pub fn entity_kind(self) -> Option<EntityKind> {
        match self {
            SemanticType::Email => Some(EntityKind::Account),
            SemanticType::Phone => Some(EntityKind::Account),
            SemanticType::Ip => Some(EntityKind::Ip),
            SemanticType::Url => Some(EntityKind::Url),
            SemanticType::Domain => Some(EntityKind::Domain),
            SemanticType::Hash => Some(EntityKind::Media),
            SemanticType::Wallet => Some(EntityKind::Wallet),
            SemanticType::PersonName => Some(EntityKind::Person),
            SemanticType::Location => Some(EntityKind::Location),
            SemanticType::Organization => Some(EntityKind::Organization),
            // A national/company id (CPF/CNPJ) is a document reference, not a
            // phone/account — Selector already covers generic identifiers
            // (IMEI/IMSI/MSISDN) and gets a document-style icon in the GUI.
            SemanticType::NationalId => Some(EntityKind::Selector),
            // money/date/bool/number/id/categorical/free-text stay as attributes
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SemanticType::Email => "email",
            SemanticType::Phone => "phone",
            SemanticType::Ip => "ip",
            SemanticType::Url => "url",
            SemanticType::Domain => "domain",
            SemanticType::Hash => "hash",
            SemanticType::Wallet => "wallet",
            SemanticType::Money => "money",
            SemanticType::Date => "date",
            SemanticType::Id => "id",
            SemanticType::PersonName => "person_name",
            SemanticType::Boolean => "boolean",
            SemanticType::Categorical => "categorical",
            SemanticType::Number => "number",
            SemanticType::FreeText => "free_text",
            SemanticType::Location => "location",
            SemanticType::Organization => "organization",
            SemanticType::NationalId => "national_id",
            SemanticType::Empty => "empty",
        }
    }
}

/// Strong header hint that overrides ambiguous value classification (a city name
/// and a person name look identical without context — the header disambiguates).
fn header_semantic(h: &str) -> Option<SemanticType> {
    let h = h.to_lowercase();
    let any = |ks: &[&str]| ks.iter().any(|k| h.contains(k));
    if any(&["city", "cidade", "country", "pais", "país", "state", "estado", "location", "local", "address", "endereço", "endereco", "region", "geo", "origin", "destination", "origem", "destino"]) {
        return Some(SemanticType::Location);
    }
    if any(&["company", "empresa", "organization", "organização", "organizacao", "org", "carrier", "vendor", "merchant", "supplier", "employer", "institution"]) {
        return Some(SemanticType::Organization);
    }
    None
}

/// Profile of one column across the sampled rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub semantic: SemanticType,
    /// 0..1 fraction of non-empty values that matched the winning type.
    pub confidence: f32,
    /// Distinct values / non-empty values (1.0 ≈ unique identifier column).
    pub uniqueness: f32,
    pub non_empty: usize,
    pub sampled: usize,
    /// EntityKind this column materializes, if any.
    pub entity_kind: Option<EntityKind>,
    /// Whether this column is the best candidate for the row's primary label.
    pub is_primary_label: bool,
}

/// Full dataset profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProfile {
    pub columns: Vec<ColumnProfile>,
    pub rows_sampled: usize,
}

impl DatasetProfile {
    /// Column-name → EntityKind mappings usable by the extractor.
    pub fn entity_mappings(&self) -> Vec<(String, EntityKind)> {
        self.columns
            .iter()
            .filter_map(|c| c.entity_kind.map(|k| (c.name.clone(), k)))
            .collect()
    }

    pub fn primary_label_column(&self) -> Option<&str> {
        self.columns.iter().find(|c| c.is_primary_label).map(|c| c.name.as_str())
    }
}

const SAMPLE: usize = 400;

/// Classify a single cell value into a semantic type (no context).
fn classify_value(v: &str) -> SemanticType {
    let t = v.trim();
    if t.is_empty() {
        return SemanticType::Empty;
    }
    // Reuse the battle-tested indicator scanners for structured types.
    let inds = extract::scan_indicators(t);
    if inds.len() == 1 {
        match inds[0].0 {
            EntityKind::Url => return SemanticType::Url,
            EntityKind::Account => return SemanticType::Email, // scan tags emails as Account
            EntityKind::Ip => return SemanticType::Ip,
            EntityKind::Media => return SemanticType::Hash,
            EntityKind::Wallet => return SemanticType::Wallet,
            EntityKind::Domain => return SemanticType::Domain,
            _ => {}
        }
    }
    if is_email(t) {
        return SemanticType::Email;
    }
    // Checked before is_phone: a CPF/CNPJ ("123.456.789-01") is digit+dot+dash
    // only — no parens/space/plus — which a real phone number almost always
    // has. Order matters, otherwise is_phone's looser check (any of
    // " +-()." with 8-15 digits) claims it first and it ends up labeled an
    // Account instead of a document.
    if is_national_id_like(t) {
        return SemanticType::NationalId;
    }
    if is_phone(t) {
        return SemanticType::Phone;
    }
    if is_money(t) {
        return SemanticType::Money;
    }
    if is_date(t) {
        return SemanticType::Date;
    }
    if is_bool(t) {
        return SemanticType::Boolean;
    }
    if is_uuid_or_id(t) {
        return SemanticType::Id;
    }
    if is_number(t) {
        return SemanticType::Number;
    }
    if is_person_name(t) {
        return SemanticType::PersonName;
    }
    SemanticType::FreeText
}

/// Profile all columns of a batch (samples up to SAMPLE rows).
pub fn profile_batch(batch: &RecordBatch) -> DatasetProfile {
    // Collect column names (union across sampled rows preserves order of first seen).
    let mut names: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let rows: Vec<&_> = batch.records.iter().take(SAMPLE).collect();
    for r in &rows {
        for k in r.fields.keys() {
            if seen.insert(k.clone()) {
                names.push(k.clone());
            }
        }
    }

    let mut columns = Vec::new();
    for name in &names {
        let mut counts: std::collections::HashMap<SemanticType, usize> = std::collections::HashMap::new();
        let mut distinct: HashSet<String> = HashSet::new();
        let mut non_empty = 0usize;
        for r in &rows {
            if let Some(v) = r.fields.get(name) {
                let st = classify_value(v);
                if st == SemanticType::Empty {
                    continue;
                }
                non_empty += 1;
                distinct.insert(v.to_lowercase());
                *counts.entry(st).or_insert(0) += 1;
            }
        }
        // Winning type = most frequent non-empty classification.
        let (winner, wc) = counts
            .iter()
            .max_by_key(|(_, c)| **c)
            .map(|(k, c)| (*k, *c))
            .unwrap_or((SemanticType::Empty, 0));
        let confidence = if non_empty > 0 { wc as f32 / non_empty as f32 } else { 0.0 };
        let uniqueness = if non_empty > 0 { distinct.len() as f32 / non_empty as f32 } else { 0.0 };

        // Header hint overrides ambiguous value types (name/free-text/categorical).
        let hdr = header_semantic(name);
        let ambiguous = matches!(winner, SemanticType::PersonName | SemanticType::FreeText | SemanticType::Categorical | SemanticType::Id);
        let semantic = if let Some(h) = hdr {
            if ambiguous { h } else { winner }
        } else if winner == SemanticType::FreeText && uniqueness < 0.15 && non_empty > 10 {
            SemanticType::Categorical
        } else if winner == SemanticType::FreeText && header_hints_name(name) {
            SemanticType::PersonName
        } else {
            winner
        };

        columns.push(ColumnProfile {
            name: name.clone(),
            semantic,
            confidence,
            uniqueness,
            non_empty,
            sampled: rows.len(),
            entity_kind: semantic.entity_kind(),
            is_primary_label: false,
        });
    }

    // Pick the primary-label column: prefer a person name, else the most unique
    // entity-bearing column (an id/email is a good row identity).
    pick_primary_label(&mut columns);

    DatasetProfile { columns, rows_sampled: rows.len() }
}

fn pick_primary_label(cols: &mut [ColumnProfile]) {
    // 1) explicit person name
    if let Some(i) = cols.iter().position(|c| c.semantic == SemanticType::PersonName) {
        cols[i].is_primary_label = true;
        return;
    }
    // 2) header hints (name/title/label)
    if let Some(i) = cols.iter().position(|c| header_hints_name(&c.name)) {
        cols[i].is_primary_label = true;
        return;
    }
    // 3) the most unique entity-bearing column (email/id), else most unique overall
    let mut best: Option<usize> = None;
    let mut best_u = -1.0f32;
    for (i, c) in cols.iter().enumerate() {
        let score = c.uniqueness + if c.entity_kind.is_some() { 0.3 } else { 0.0 };
        if c.non_empty > 0 && score > best_u {
            best_u = score;
            best = Some(i);
        }
    }
    if let Some(i) = best {
        cols[i].is_primary_label = true;
    }
}

fn header_hints_name(h: &str) -> bool {
    let h = h.to_lowercase();
    ["name", "full name", "fullname", "nome", "display name", "title", "label", "subject"]
        .iter()
        .any(|k| h == *k || h.contains("name") || h.contains("nome"))
}

// ---------------------------------------------------------------------------
// Value classifiers (dependency-free; complement extract.rs scanners).
// ---------------------------------------------------------------------------

fn is_email(t: &str) -> bool {
    let at = t.matches('@').count();
    at == 1 && !t.starts_with('@') && !t.ends_with('@') && t.split('@').nth(1).map(|d| d.contains('.')).unwrap_or(false)
}

fn is_phone(t: &str) -> bool {
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    let ok_chars = t.chars().all(|c| c.is_ascii_digit() || " +-().".contains(c));
    ok_chars && digits >= 8 && digits <= 15 && t.chars().any(|c| !c.is_ascii_digit() || true)
}

/// CPF (11 digits) or CNPJ (14 digits) shape: digits with ONLY dots/dashes as
/// separators (no parens/space/plus, which real phone numbers almost always
/// have) — e.g. "123.456.789-01" or "12.345.678/0001-90".
fn is_national_id_like(t: &str) -> bool {
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    let ok_chars = t.chars().all(|c| c.is_ascii_digit() || ".-/".contains(c));
    ok_chars && (digits == 11 || digits == 14) && (t.contains('.') || t.contains('-'))
}

fn is_money(t: &str) -> bool {
    let t = t.trim();
    let has_sym = t.starts_with('$') || t.starts_with('R') && t.contains('$') || t.starts_with('€') || t.starts_with('£');
    let body: String = t.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    let digits = body.chars().filter(|c| c.is_ascii_digit()).count();
    has_sym && digits >= 1
}

fn is_date(t: &str) -> bool {
    // ISO-ish or common separators; require digits + separators, not a plain int.
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    if digits < 4 {
        return false;
    }
    let has_sep = t.contains('-') || t.contains('/') || t.contains(':') || t.contains('T');
    let alpha = t.chars().filter(|c| c.is_ascii_alphabetic()).count();
    has_sep && alpha <= 3 && t.len() >= 6 && t.len() <= 40
}

fn is_bool(t: &str) -> bool {
    matches!(t.to_lowercase().as_str(), "true" | "false" | "yes" | "no" | "y" | "n" | "sim" | "não" | "nao" | "0" | "1" if t.len() <= 5)
}

fn is_uuid_or_id(t: &str) -> bool {
    // UUID
    let dashes = t.matches('-').count();
    let hexish = t.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if dashes == 4 && t.len() == 36 && hexish {
        return true;
    }
    // long alnum token with no spaces, mixed digits+letters → opaque id
    if t.len() >= 8 && !t.contains(' ') && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        let d = t.chars().filter(|c| c.is_ascii_digit()).count();
        let a = t.chars().filter(|c| c.is_ascii_alphabetic()).count();
        return d >= 1 && a >= 1;
    }
    false
}

fn is_number(t: &str) -> bool {
    let t = t.replace(',', "");
    t.parse::<f64>().is_ok()
}

fn is_person_name(t: &str) -> bool {
    // 1–4 words, each capitalized-ish, alphabetic + spaces, not too long.
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 4 || t.len() > 48 {
        return false;
    }
    let alpha = t.chars().filter(|c| c.is_alphabetic()).count();
    let non_alpha = t.chars().filter(|c| !c.is_alphabetic() && !c.is_whitespace() && *c != '.' && *c != '\'' && *c != '-').count();
    alpha >= 2 && non_alpha == 0 && words.iter().all(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) || w.len() <= 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{Record, RecordBatch};
    use indexmap::IndexMap;

    fn rec(pairs: &[(&str, &str)], i: usize) -> Record {
        let mut fields = IndexMap::new();
        for (k, v) in pairs {
            fields.insert(k.to_string(), v.to_string());
        }
        Record { fields, origin: "test.csv".into(), index: i }
    }

    #[test]
    fn infers_ontology_from_unknown_headers() {
        // No known schema words — must still discover semantic types.
        let batch = RecordBatch {
            records: vec![
                rec(&[("holder", "Maria Silva"), ("contact", "maria@ex.com"), ("box", "acct-1"), ("logged_from", "203.0.113.5")], 0),
                rec(&[("holder", "John Carter"), ("contact", "john@ex.com"), ("box", "acct-2"), ("logged_from", "203.0.113.6")], 1),
            ],
            declared_type: None,
            origin: "test.csv".into(),
        };
        let p = profile_batch(&batch);
        let get = |n: &str| p.columns.iter().find(|c| c.name == n).unwrap();
        assert_eq!(get("holder").semantic, SemanticType::PersonName);
        assert_eq!(get("holder").entity_kind, Some(EntityKind::Person));
        assert_eq!(get("contact").semantic, SemanticType::Email);
        assert_eq!(get("contact").entity_kind, Some(EntityKind::Account));
        assert_eq!(get("logged_from").semantic, SemanticType::Ip);
        // at least one column must be chosen as the primary label
        assert!(p.columns.iter().any(|c| c.is_primary_label));
    }

    #[test]
    fn header_hint_disambiguates_city_from_name() {
        let batch = RecordBatch {
            records: vec![
                rec(&[("origin_city", "Sao Paulo"), ("company", "Acme Ltd")], 0),
                rec(&[("origin_city", "Miami"), ("company", "Globex")], 1),
            ],
            declared_type: None,
            origin: "t.csv".into(),
        };
        let p = profile_batch(&batch);
        let get = |n: &str| p.columns.iter().find(|c| c.name == n).unwrap();
        assert_eq!(get("origin_city").entity_kind, Some(EntityKind::Location));
        assert_eq!(get("company").entity_kind, Some(EntityKind::Organization));
    }

    #[test]
    fn value_classifiers() {
        assert_eq!(classify_value("$1,250.00"), SemanticType::Money);
        assert_eq!(classify_value("2026-01-05T10:11:00Z"), SemanticType::Date);
        assert_eq!(classify_value("maria@ex.com"), SemanticType::Email);
        assert_eq!(classify_value("203.0.113.5"), SemanticType::Ip);
        assert_eq!(classify_value(""), SemanticType::Empty);
    }
}
