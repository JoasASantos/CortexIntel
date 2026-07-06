//! Importable plugins. A plugin is a small JSON manifest that extends the
//! platform without recompiling: it can register extra classification keywords,
//! entity-field mappings, risk signals and prompt add-ons for a vertical.
//! Plugins live in `~/.cortexintel/plugins/*.json` and are loaded at startup.

use crate::store;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// Which verticals this plugin applies to (empty = all).
    #[serde(default)]
    pub domains: Vec<String>,
    /// Extra entity-field mappings: column name -> entity kind.
    #[serde(default)]
    pub field_mappings: Vec<FieldMapping>,
    /// Extra risk signal tokens -> weight.
    #[serde(default)]
    pub risk_signals: Vec<RiskSignal>,
    /// Extra text appended to agent system prompts for this plugin.
    #[serde(default)]
    pub prompt_addon: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub field: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSignal {
    pub token: String,
    pub weight: f32,
}

/// List all installed plugins.
pub fn list() -> Vec<Plugin> {
    let dir = store::plugins_dir();
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.path().extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(e.path()) {
                if let Ok(p) = serde_json::from_str::<Plugin>(&s) {
                    out.push(p);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Install a plugin from its manifest JSON (validates + assigns id if missing).
pub fn install(manifest: &str) -> Result<Plugin> {
    let mut p: Plugin = serde_json::from_str(manifest).map_err(|e| anyhow!("invalid plugin manifest: {e}"))?;
    if p.name.trim().is_empty() {
        return Err(anyhow!("plugin manifest needs a 'name'"));
    }
    if p.id.trim().is_empty() {
        p.id = format!("plg-{}", uuid::Uuid::new_v4().simple());
    }
    if !p.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(anyhow!("invalid plugin id"));
    }
    p.enabled = true;
    store::write_json(&store::plugins_dir().join(format!("{}.json", p.id)), &p)?;
    Ok(p)
}

pub fn set_enabled(id: &str, enabled: bool) -> Result<()> {
    let path = store::plugins_dir().join(format!("{id}.json"));
    let mut p: Plugin = serde_json::from_str(&std::fs::read_to_string(&path).map_err(|_| anyhow!("plugin not found"))?)?;
    p.enabled = enabled;
    store::write_json(&path, &p)
}

pub fn remove(id: &str) -> Result<()> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(anyhow!("invalid plugin id"));
    }
    std::fs::remove_file(store::plugins_dir().join(format!("{id}.json"))).map_err(|e| anyhow!("cannot remove: {e}"))
}

/// Enabled plugins that apply to a domain — consumed by the extraction/risk layers.
pub fn active_for(domain: &str) -> Vec<Plugin> {
    list()
        .into_iter()
        .filter(|p| p.enabled && (p.domains.is_empty() || p.domains.iter().any(|d| d == domain)))
        .collect()
}
