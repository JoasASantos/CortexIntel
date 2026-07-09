//! Deterministic, offline entity/relationship extraction. This is the backbone
//! of ingestion: it maps DATA.md field names onto ontology kinds and scans free
//! text for indicators (IP, URL, email, hash, wallet, domain, phone). The LLM
//! extractor augments — it never has to run for the pipeline to produce a graph.

use crate::config::DataType;
use crate::ontology::{Entity, EntityKind};
use crate::sources::Record;

/// A relationship expressed by entity labels (resolved to ids after upsert).
#[derive(Debug, Clone)]
pub struct LabelLink {
    pub source_label: String,
    pub rel_type: String,
    pub target_label: String,
    pub confidence: f32,
}

/// Output of extracting one record.
#[derive(Debug, Default)]
pub struct ExtractResult {
    pub entities: Vec<Entity>,
    pub links: Vec<LabelLink>,
}

/// Known DATA.md field name -> (EntityKind, canonical attribute name).
/// The first matching field for a kind becomes that entity's label.
const FIELD_MAP: &[(&[&str], EntityKind)] = &[
    (&["case_id", "case_title"], EntityKind::Case),
    (&["report_id", "platform_report_id"], EntityKind::Report),
    (&["victim_id"], EntityKind::Victim),
    (&["suspect_id"], EntityKind::Suspect),
    (&["full_name", "name", "person_id", "user_id", "user id", "student_id", "student id", "customer_id", "employee_id"], EntityKind::Person),
    (&["account_id", "username", "email", "discord_username", "discord username", "platform_user_id", "discord_user_id", "discord user id"], EntityKind::Account),
    (&["device_id", "device_identifier_hash", "hostname"], EntityKind::Device),
    (&["ip_address"], EntityKind::Ip),
    (&["url_id", "full_url", "normalized_url", "profile_url"], EntityKind::Url),
    (&["domain"], EntityKind::Domain),
    (&["media_id", "sha256", "perceptual_hash"], EntityKind::Media),
    (&["evidence_id"], EntityKind::Evidence),
    (&["communication_id", "conversation_id"], EntityKind::Communication),
    (&["group_id", "group_name"], EntityKind::Group),
    (&["wallet_address", "wallet_cluster_id"], EntityKind::Wallet),
    (&["financial_event_id", "transaction_id"], EntityKind::Payment),
    (&["location_id", "latitude_approx", "city"], EntityKind::Location),
    (&["provider_record_id", "provider_name"], EntityKind::Organization),
    (&["vulnerability", "cve"], EntityKind::Vulnerability),
    (&["incident_id", "network_event_id"], EntityKind::Incident),
];

/// Extract entities + relationships from a single record. `extra` carries
/// plugin-provided field→kind mappings applied on top of the built-in map.
pub fn extract_record(rec: &Record, _dt: DataType, extra: &[(String, EntityKind)]) -> ExtractResult {
    let mut out = ExtractResult::default();
    let origin = format!("{}#{}", rec.origin, rec.index);

    // 1) Map declared fields onto ontology entities.
    let mut by_kind: Vec<(EntityKind, String)> = Vec::new();
    for (keys, kind) in FIELD_MAP {
        if let Some(val) = rec.get_any(keys) {
            let label = safe_label(*kind, val);
            let mut e = Entity::new(*kind, &label).with_source(&origin);
            // Attach a few descriptive attributes if present.
            attach_attrs(&mut e, rec);
            // Preserve the raw file hash (a one-way fingerprint, safe to keep and
            // to match against reference feeds) — the label itself is redacted.
            attach_hash(&mut e, val);
            // Media also carries a perceptual hash when present: matched by
            // similarity (near-duplicate), so recompressed/altered copies still hit.
            if *kind == EntityKind::Media {
                if let Some(ph) = rec.get_any(&["perceptual_hash", "phash", "p_hash"]) {
                    let ph = ph.trim().to_lowercase();
                    if !ph.is_empty() {
                        e.attributes.entry("perceptual_hash".into()).or_insert(ph);
                    }
                }
            }
            out.entities.push(e);
            by_kind.push((*kind, label));
        }
    }
    // 1b) Plugin/auto-ontology field mappings — provenance includes the column.
    for (field, kind) in extra {
        if let Some(val) = rec.get_any(&[field.as_str()]) {
            let label = safe_label(*kind, val);
            if !by_kind.iter().any(|(k, l)| k == kind && *l == label) {
                let e = Entity::new(*kind, &label)
                    .with_source(&format!("{origin}:{field}"))
                    .with_attr("derived_from_column", field.clone());
                out.entities.push(e);
                by_kind.push((*kind, label));
            }
        }
    }

    // 2) Scan every value for indicators (covers undeclared/free-text feeds).
    for (_k, v) in &rec.fields {
        for (kind, raw) in scan_indicators(v) {
            let label = safe_label(kind, &raw);
            if !by_kind.iter().any(|(k, l)| *k == kind && *l == label) {
                let mut e = Entity::new(kind, &label).with_source(&origin);
                attach_hash(&mut e, &raw);
                out.entities.push(e);
                by_kind.push((kind, label));
            }
        }
    }

    // 2b) For email accounts, materialize the email DOMAIN as its own entity and
    // link the account to it. This is what lets the graph answer "how many are
    // @gmail.com?" (the domain node's degree) and cluster accounts by provider.
    let mut email_domains: Vec<(String, String)> = Vec::new(); // (account_label, domain)
    for e in &out.entities {
        if e.kind == EntityKind::Account {
            if let Some((_, dom)) = e.label.split_once('@') {
                let dom = dom.trim().to_lowercase();
                if !dom.is_empty() && dom.contains('.') {
                    email_domains.push((e.label.clone(), dom));
                }
            }
        }
    }
    for (acct, dom) in email_domains {
        if !by_kind.iter().any(|(k, l)| *k == EntityKind::Domain && *l == dom) {
            out.entities.push(Entity::new(EntityKind::Domain, &dom).with_source(&origin));
            by_kind.push((EntityKind::Domain, dom.clone()));
        }
        out.links.push(LabelLink { source_label: acct, rel_type: "uses_email_domain".into(), target_label: dom, confidence: 0.9 });
    }

    // 3) Intra-record relationships from co-occurrence (DATA.md relations).
    out.links.extend(infer_links(&by_kind));

    out
}

/// If `raw` is a file hash, retain it (lowercased) plus its type as attributes.
/// A hash is a one-way fingerprint — safe to keep and to match against known-hash
/// reference feeds (malware sets, known-CSAM hash lists, etc.), unlike the file.
fn attach_hash(e: &mut Entity, raw: &str) {
    let t = raw.trim();
    if !is_hash(t) {
        return;
    }
    let kind = match t.len() {
        32 => "md5",
        40 => "sha1",
        64 => "sha256",
        _ => "hash",
    };
    e.attributes.entry("file_hash".into()).or_insert_with(|| t.to_lowercase());
    e.attributes.entry("hash_type".into()).or_insert_with(|| kind.to_string());
}

fn attach_attrs(e: &mut Entity, rec: &Record) {
    // Copy a handful of common descriptive fields when present, without dumping
    // the whole (possibly sensitive) row.
    const SAFE: &[&str] = &[
        "case_type", "case_status", "case_priority", "urgency_level", "source_type",
        "platform_name", "risk_level", "confidence_level", "status", "country",
        "jurisdiction_country", "account_status", "device_type", "media_type",
        "transaction_type", "url_type", "group_type", "report_category",
        "student_type", "student type", "created_at", "created at", "role",
        "customer_type", "order_status", "product_category", "department",
        // Monetary: needed so payments/wallets/accounts carry their value.
        "amount", "value", "currency", "transaction_amount", "total", "balance",
        // Discipline signals: HUMINT reliability grading reads these.
        "source_reliability", "reliability", "credibility", "corroboration",
        "source_grade", "info_grade", "reporter_type",
        // Geo / temporal / trajectory: needed by the map lens to plot entities
        // and draw movement paths. Coordinates are references, not raw content.
        "latitude", "longitude", "lat", "lon", "lng", "latitude_approx", "longitude_approx",
        "gpslatitude", "gpslongitude", "city", "location", "trajectory", "track",
        "vessel", "subject", "timestamp", "first_seen_at", "observed_at", "received_at", "date",
    ];
    for k in SAFE {
        if let Some(v) = rec.get_any(&[k]) {
            e.attributes.insert((*k).to_string(), v.to_string());
        }
    }
}

/// Build relationships between kinds observed together in one record.
fn infer_links(by_kind: &[(EntityKind, String)]) -> Vec<LabelLink> {
    use EntityKind::*;
    let find = |k: EntityKind| by_kind.iter().find(|(kk, _)| *kk == k).map(|(_, l)| l.clone());
    let mut links = Vec::new();
    let mut link = |s: Option<String>, rel: &str, t: Option<String>, c: f32| {
        if let (Some(s), Some(t)) = (s, t) {
            if s != t {
                links.push(LabelLink {
                    source_label: s,
                    rel_type: rel.into(),
                    target_label: t,
                    confidence: c,
                });
            }
        }
    };

    link(find(Case), "has_report", find(Report), 0.95);
    link(find(Case), "has_evidence", find(Evidence), 0.95);
    link(find(Person), "owns_account", find(Account), 0.8);
    link(find(Suspect), "uses_account", find(Account), 0.75);
    link(find(Account), "uses_device", find(Device), 0.7);
    link(find(Account), "logged_in_from_ip", find(Ip), 0.7);
    link(find(Account), "member_of_group", find(Group), 0.65);
    link(find(Account), "sent", find(Communication), 0.6);
    link(find(Communication), "contains_url", find(Url), 0.6);
    link(find(Url), "belongs_to_domain", find(Domain), 0.85);
    link(find(Url), "hosts_media", find(Media), 0.55);
    link(find(Account), "paid", find(Payment), 0.6);
    link(find(Payment), "to_wallet", find(Wallet), 0.7);
    link(find(Suspect), "contacted", find(Victim), 0.6);
    link(find(Device), "observed_at", find(Location), 0.5);
    link(find(Media), "part_of_case", find(Case), 0.8);
    link(find(Report), "references_account", find(Account), 0.6);
    link(find(Report), "references_url", find(Url), 0.6);

    links
}

/// Produce a display-safe label. Sensitive kinds are referenced, not shown raw.
pub fn safe_label(kind: EntityKind, raw: &str) -> String {
    let raw = raw.trim();
    if kind.is_sensitive() {
        // Reference by short fingerprint rather than raw value.
        let short = short_ref(raw);
        format!("{}:{}", kind.as_str(), short)
    } else {
        // Truncate very long labels.
        if raw.chars().count() > 80 {
            format!("{}…", raw.chars().take(79).collect::<String>())
        } else {
            raw.to_string()
        }
    }
}

fn short_ref(s: &str) -> String {
    // Non-cryptographic short reference; keeps sensitive raw values out of labels.
    let mut h: u64 = 1469598103934665603;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{:012x}", h & 0xffff_ffff_ffff)
}

// ---------------------------------------------------------------------------
// Indicator scanners (dependency-free).
// ---------------------------------------------------------------------------

/// Scan a string and return (kind, value) indicators found in it.
pub fn scan_indicators(s: &str) -> Vec<(EntityKind, String)> {
    let mut found = Vec::new();
    for tok in s.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '|') {
        let t = tok.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != ':' && c != '/' && c != '@' && c != '-' && c != '_');
        if t.is_empty() {
            continue;
        }
        if is_url(t) {
            found.push((EntityKind::Url, t.to_string()));
        } else if is_email(t) {
            found.push((EntityKind::Account, t.to_string()));
        } else if is_ipv4(t) {
            found.push((EntityKind::Ip, t.to_string()));
        } else if is_hash(t) {
            found.push((EntityKind::Media, t.to_string()));
        } else if is_eth_wallet(t) {
            found.push((EntityKind::Wallet, t.to_string()));
        } else if is_domain(t) {
            found.push((EntityKind::Domain, t.to_string()));
        }
    }
    found
}

fn is_url(t: &str) -> bool {
    t.starts_with("http://") || t.starts_with("https://") || t.starts_with("www.")
}

fn is_email(t: &str) -> bool {
    let at = t.matches('@').count();
    at == 1
        && !t.starts_with('@')
        && !t.ends_with('@')
        && t.split('@').nth(1).map(|d| d.contains('.')).unwrap_or(false)
}

fn is_ipv4(t: &str) -> bool {
    let parts: Vec<&str> = t.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u16>().map(|n| n <= 255).unwrap_or(false))
}

fn is_hash(t: &str) -> bool {
    let n = t.len();
    (n == 32 || n == 40 || n == 64) && t.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_eth_wallet(t: &str) -> bool {
    t.len() == 42 && t.starts_with("0x") && t[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_domain(t: &str) -> bool {
    // token.tld, no scheme, no @, has a dot, alnum + dash + dot only, TLD >= 2 alpha.
    if t.contains('@') || t.contains('/') || t.contains(':') {
        return false;
    }
    let labels: Vec<&str> = t.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().unwrap();
    tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
        && labels
            .iter()
            .all(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}
