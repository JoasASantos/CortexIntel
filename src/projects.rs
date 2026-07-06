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
    };
    save(&p)?;
    Ok(p)
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
