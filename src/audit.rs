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
    pub events: Vec<AuditEvent>,
}

impl AuditLog {
    pub fn new(dir: &Path, operator: &str, legal_basis: &str) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(AuditLog {
            path: dir.join("audit.log.jsonl"),
            operator: operator.to_string(),
            legal_basis: legal_basis.to_string(),
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
        let ev = AuditEvent {
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
        };
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
