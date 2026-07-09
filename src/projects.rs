//! Projects. Each project is a named investigation workspace with its own
//! vertical, saved connectors, activity log and last analysis result. Projects
//! persist as one JSON file each under `~/.cortexintel/projects/` and can be
//! exported/imported as a self-contained bundle (the same JSON).

use crate::store;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub kind: String, // "run" | "import" | "connect" | "note"
    pub summary: String,
    pub at: u64,
    #[serde(default)]
    pub meta: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConnector {
    pub id: String,
    pub kind: String, // csv | json | postgres | mysql | bigquery | datalake | mcp
    pub name: String,
    /// Non-secret config (host, db, path…). Secrets are NOT persisted here.
    #[serde(default)]
    pub config: serde_json::Value,
    pub added_at: u64,
}

/// G5 — an analyst comment attached to any object in the case (the situation,
/// an entity, or a decision). Persisted with the project for need-to-know
/// collaboration (live multiplayer is intentionally out of scope).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    /// Object this comment is about: "situation", or an entity/decision id.
    pub object_id: String,
    /// "situation" | "entity" | "decision".
    pub object_kind: String,
    pub author: String,
    pub text: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub owner: String,
    #[serde(default)]
    pub description: String,
    /// Optional operator instructions steering the AI for this project.
    #[serde(default)]
    pub ai_instructions: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub activities: Vec<Activity>,
    #[serde(default)]
    pub connectors: Vec<SavedConnector>,
    /// Last consolidated analysis document (graph, risk, brief…).
    #[serde(default)]
    pub last_result: Option<serde_json::Value>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    /// Standing watchlist rules (continuous intelligence): each fires an alert on
    /// a re-run when matched. Rule = {name, kind?, min_risk?, label_contains?}.
    #[serde(default)]
    pub watchlist: Vec<serde_json::Value>,
    /// Per-case RBAC (need-to-know): when `restricted`, only the owner, listed
    /// `members` (by email) and admins can access the case.
    #[serde(default)]
    pub restricted: bool,
    #[serde(default)]
    pub members: Vec<String>,
}

/// Can this user access the project? Owner, listed member or admin always can;
/// an unrestricted project is open to any signed-in user.
pub fn can_access(p: &Project, email: &str, role: &str) -> bool {
    if role == "admin" || p.owner.eq_ignore_ascii_case(email) {
        return true;
    }
    if !p.restricted {
        return true;
    }
    p.members.iter().any(|m| m.eq_ignore_ascii_case(email))
}

/// Set a project's access control (owner/admin only — enforced at the API).
pub fn set_access(id: &str, restricted: bool, members: Vec<String>) -> Result<()> {
    let mut p = load(id)?;
    p.restricted = restricted;
    p.members = members;
    p.updated_at = now();
    save(&p)
}

/// List only the projects a user may see.
pub fn list_for(email: &str, role: &str) -> Vec<ProjectSummary> {
    list()
        .into_iter()
        .filter(|s| load(&s.id).map(|p| can_access(&p, email, role)).unwrap_or(true))
        .collect()
}

/// Lightweight listing entry (no heavy last_result payload).
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub updated_at: u64,
    pub activity_count: usize,
    pub connector_count: usize,
    pub has_result: bool,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn path_for(id: &str) -> std::path::PathBuf {
    store::projects_dir().join(format!("{id}.json"))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn create(name: &str, domain: &str, owner: &str, description: &str, ai_instructions: &str) -> Result<Project> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("project name is required"));
    }
    let t = now();
    let p = Project {
        id: format!("prj-{}", uuid::Uuid::new_v4().simple()),
        name: name.to_string(),
        domain: domain.to_string(),
        owner: owner.to_string(),
        description: description.trim().to_string(),
        ai_instructions: ai_instructions.trim().to_string(),
        created_at: t,
        updated_at: t,
        activities: vec![Activity {
            id: format!("act-{}", uuid::Uuid::new_v4().simple()),
            kind: "note".into(),
            summary: "Project created".into(),
            at: t,
            meta: serde_json::Value::Null,
        }],
        connectors: Vec::new(),
        last_result: None,
        comments: Vec::new(),
        watchlist: Vec::new(),
        restricted: false,
        members: Vec::new(),
    };
    save(&p)?;
    Ok(p)
}

/// Replace a project's watchlist rules (continuous intelligence).
pub fn set_watchlist(id: &str, rules: Vec<serde_json::Value>) -> Result<()> {
    let mut p = load(id)?;
    p.watchlist = rules;
    p.updated_at = now();
    save(&p)
}

pub fn save(p: &Project) -> Result<()> {
    if !valid_id(&p.id) {
        return Err(anyhow!("invalid project id"));
    }
    store::write_json(&path_for(&p.id), p)
}

pub fn load(id: &str) -> Result<Project> {
    if !valid_id(id) {
        return Err(anyhow!("invalid project id"));
    }
    let raw = std::fs::read_to_string(path_for(id)).map_err(|_| anyhow!("project not found"))?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn delete(id: &str) -> Result<()> {
    if !valid_id(id) {
        return Err(anyhow!("invalid project id"));
    }
    std::fs::remove_file(path_for(id)).map_err(|e| anyhow!("cannot delete: {e}"))
}

pub fn list() -> Vec<ProjectSummary> {
    let dir = store::projects_dir();
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(entry.path()) {
                if let Ok(p) = serde_json::from_str::<Project>(&s) {
                    out.push(ProjectSummary {
                        id: p.id,
                        name: p.name,
                        domain: p.domain,
                        updated_at: p.updated_at,
                        activity_count: p.activities.len(),
                        connector_count: p.connectors.len(),
                        has_result: p.last_result.is_some(),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

pub fn add_activity(id: &str, kind: &str, summary: &str, meta: serde_json::Value) -> Result<()> {
    let mut p = load(id)?;
    p.activities.push(Activity {
        id: format!("act-{}", uuid::Uuid::new_v4().simple()),
        kind: kind.to_string(),
        summary: summary.to_string(),
        at: now(),
        meta,
    });
    p.updated_at = now();
    save(&p)
}

/// G5 — add an analyst comment on an object in the project. Returns the comment.
pub fn add_comment(id: &str, object_id: &str, object_kind: &str, author: &str, text: &str) -> Result<Comment> {
    let text = text.trim();
    if text.is_empty() {
        return Err(anyhow!("comment text is empty"));
    }
    let mut p = load(id)?;
    let c = Comment {
        id: format!("cmt-{}", uuid::Uuid::new_v4().simple()),
        object_id: object_id.to_string(),
        object_kind: object_kind.to_string(),
        author: author.to_string(),
        text: text.to_string(),
        created_at: now(),
    };
    p.comments.push(c.clone());
    p.updated_at = now();
    save(&p)?;
    Ok(c)
}

/// List comments for a project, optionally filtered to one object.
pub fn list_comments(id: &str, object_id: Option<&str>) -> Result<Vec<Comment>> {
    let p = load(id)?;
    Ok(p.comments.into_iter().filter(|c| object_id.map_or(true, |o| c.object_id == o)).collect())
}

pub fn set_result(id: &str, result: serde_json::Value) -> Result<()> {
    let mut p = load(id)?;
    p.last_result = Some(result);
    p.updated_at = now();
    save(&p)
}

pub fn add_connector(id: &str, kind: &str, name: &str, config: serde_json::Value) -> Result<SavedConnector> {
    let mut p = load(id)?;
    let c = SavedConnector {
        id: format!("con-{}", uuid::Uuid::new_v4().simple()),
        kind: kind.to_string(),
        name: name.to_string(),
        config,
        added_at: now(),
    };
    p.connectors.push(c.clone());
    p.updated_at = now();
    save(&p)?;
    Ok(c)
}

/// Export a project as its bundle JSON (string).
pub fn export(id: &str) -> Result<String> {
    let p = load(id)?;
    Ok(serde_json::to_string_pretty(&p)?)
}

/// Import a project bundle, assigning a fresh id to avoid collisions.
pub fn import(bundle: &str, owner: &str) -> Result<Project> {
    let mut p: Project = serde_json::from_str(bundle).map_err(|e| anyhow!("invalid bundle: {e}"))?;
    p.id = format!("prj-{}", uuid::Uuid::new_v4().simple());
    p.owner = owner.to_string();
    p.updated_at = now();
    p.name = format!("{} (imported)", p.name);
    save(&p)?;
    Ok(p)
}
