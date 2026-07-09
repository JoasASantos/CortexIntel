//! Audit, retention and legal-disposal support (DATA.md §26, §20). Every stage
//! writes an append-only audit event; the run records a retention policy and a
//! computed disposal date so downstream governance can enforce it.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// One append-only audit event (DATA.md §26).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub audit_event_id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub action_performed: String,
    pub stage: String,
    pub entity_scope: String,
    pub reason_for_access: String,
    pub legal_basis: String,
    pub sensitive_data_viewed: bool,
    pub export_performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Chain of custody: hash of the previous event ("" / genesis for the first).
    #[serde(default)]
    pub prev_hash: String,
    /// Hash of THIS event's content chained onto prev_hash — tamper-evident.
    #[serde(default)]
    pub hash: String,
}

/// A small dependency-free content hash (FNV-1a, 64-bit, hex). Not cryptographic,
/// but chained it makes any edit/reorder/deletion of the audit trail detectable.
fn chain_hash(prev: &str, content: &str) -> String {
    let mut h: u64 = 1469598103934665603;
    for b in prev.as_bytes().iter().chain(content.as_bytes()) {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{h:016x}")
}

/// Retention policy attached to a run (DATA.md §4 retention/disposal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub created_at: DateTime<Utc>,
    pub retention_days: i64,
    pub disposal_date: DateTime<Utc>,
    pub legal_basis: String,
    pub note: String,
}

impl RetentionPolicy {
    pub fn new(now: DateTime<Utc>, retention_days: i64, legal_basis: &str) -> Self {
        RetentionPolicy {
            created_at: now,
            retention_days,
            disposal_date: now + Duration::days(retention_days),
            legal_basis: legal_basis.to_string(),
            note: format!(
                "Records and derived graph are subject to disposal on or after {} unless a legal hold applies.",
                (now + Duration::days(retention_days)).format("%Y-%m-%d")
            ),
        }
    }
}

/// Append-only audit log writer + in-memory mirror for the final report.
pub struct AuditLog {
    path: PathBuf,
    operator: String,
    legal_basis: String,
    last_hash: String,
    pub events: Vec<AuditEvent>,
}

impl AuditLog {
    pub fn new(dir: &Path, operator: &str, legal_basis: &str) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(AuditLog {
            path: dir.join("audit.log.jsonl"),
            operator: operator.to_string(),
            legal_basis: legal_basis.to_string(),
            last_hash: "genesis".to_string(),
            events: Vec::new(),
        })
    }

    /// Record an event both to the in-memory list and the append-only file.
    pub fn record(
        &mut self,
        now: DateTime<Utc>,
        action: &str,
        stage: &str,
        scope: &str,
        reason: &str,
        sensitive: bool,
        export: bool,
        provider: Option<String>,
        model: Option<String>,
    ) {
        let mut ev = AuditEvent {
            audit_event_id: format!("aud-{}", uuid::Uuid::new_v4().simple()),
            timestamp: now,
            user_id: self.operator.clone(),
            action_performed: action.to_string(),
            stage: stage.to_string(),
            entity_scope: scope.to_string(),
            reason_for_access: reason.to_string(),
            legal_basis: self.legal_basis.clone(),
            sensitive_data_viewed: sensitive,
            export_performed: export,
            provider,
            model,
            prev_hash: self.last_hash.clone(),
            hash: String::new(),
        };
        // Chain the content (everything except the hash field itself) onto prev.
        let content = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            ev.audit_event_id, ev.timestamp.to_rfc3339(), ev.user_id,
            ev.action_performed, ev.stage, ev.entity_scope, ev.sensitive_data_viewed
        );
        ev.hash = chain_hash(&ev.prev_hash, &content);
        self.last_hash = ev.hash.clone();
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            if let Ok(line) = serde_json::to_string(&ev) {
                let _ = writeln!(f, "{line}");
            }
        }
        self.events.push(ev);
    }

    pub fn sensitive_touch_count(&self) -> usize {
        self.events.iter().filter(|e| e.sensitive_data_viewed).count()
    }
}

/// Result of verifying an audit chain of custody.
#[derive(Debug, Clone, Serialize)]
pub struct ChainVerdict {
    pub ok: bool,
    pub events: usize,
    /// 1-indexed position of the first broken event, if any.
    pub broken_at: Option<usize>,
    pub message: String,
}

/// Verify the chain of custody of an `audit.log.jsonl`: recompute each event's
/// hash and confirm it links to the previous. Any edit, reorder or deletion
/// breaks the chain and is reported.
pub fn verify_chain(path: &Path) -> ChainVerdict {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return ChainVerdict { ok: false, events: 0, broken_at: None, message: format!("cannot read audit log: {e}") },
    };
    let mut prev = "genesis".to_string();
    let mut n = 0usize;
    for (i, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let ev: AuditEvent = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return ChainVerdict { ok: false, events: n, broken_at: Some(i + 1), message: format!("event {} is not valid JSON", i + 1) },
        };
        n += 1;
        let content = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            ev.audit_event_id, ev.timestamp.to_rfc3339(), ev.user_id,
            ev.action_performed, ev.stage, ev.entity_scope, ev.sensitive_data_viewed
        );
        let expect = chain_hash(&prev, &content);
        if ev.prev_hash != prev {
            return ChainVerdict { ok: false, events: n, broken_at: Some(i + 1), message: format!("event {} does not link to the previous (broken/reordered/deleted)", i + 1) };
        }
        if ev.hash != expect {
            return ChainVerdict { ok: false, events: n, broken_at: Some(i + 1), message: format!("event {} content was altered (hash mismatch)", i + 1) };
        }
        prev = ev.hash;
    }
    ChainVerdict { ok: true, events: n, broken_at: None, message: format!("chain of custody intact — {n} event(s) verified") }
}
