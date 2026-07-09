//! Local persistence root. CortexIntel keeps no analytical database, but it does
//! need a small home for accounts, projects and plugins. Everything lives under
//! `~/.cortexintel/` (override with `CORTEX_HOME_DIR`), with restrictive perms.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Base data directory, created on first use. Resolves to the first location
/// that is (or can be made) writable, so the desktop app never fails to persist
/// just because one candidate path is not writable in its launch context.
/// Order: `CORTEX_HOME_DIR` → `~/.cortexintel` → OS app-data → temp.
pub fn base_dir() -> PathBuf {
    use std::sync::OnceLock;
    static RESOLVED: OnceLock<PathBuf> = OnceLock::new();
    RESOLVED
        .get_or_init(|| {
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Ok(p) = std::env::var("CORTEX_HOME_DIR") {
                candidates.push(PathBuf::from(p));
            }
            if let Some(h) = dirs::home_dir() {
                candidates.push(h.join(".cortexintel"));
            }
            if let Some(d) = dirs::data_dir() {
                candidates.push(d.join("CortexIntel"));
            }
            candidates.push(std::env::temp_dir().join("cortexintel"));

            for c in &candidates {
                if is_writable(c) {
                    return c.clone();
                }
            }
            // Last resort: temp (should always work).
            std::env::temp_dir().join("cortexintel")
        })
        .clone()
}

/// True if the directory exists and is writable, or can be created + written to.
fn is_writable(dir: &std::path::Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".cortex_write_probe");
    match std::fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

pub fn ensure_dir(p: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(p)
        .with_context(|| format!("cannot create data directory {} (permission denied?). Set CORTEX_HOME_DIR to a writable path.", p.display()))?;
    harden(p);
    Ok(())
}

pub fn users_file() -> PathBuf {
    base_dir().join("users.json")
}

pub fn projects_dir() -> PathBuf {
    base_dir().join("projects")
}

pub fn plugins_dir() -> PathBuf {
    base_dir().join("plugins")
}

pub fn uploads_dir() -> PathBuf {
    base_dir().join("uploads")
}

fn settings_file() -> PathBuf {
    base_dir().join("settings.json")
}

/// Instance settings (country for locale-aware KYC checks, onboarding state).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub country: String, // "BR" | "US" | ""
    #[serde(default)]
    pub onboarded: bool,
    /// Tenant name (company / business unit / team) set on first access.
    #[serde(default)]
    pub organization: String,
    /// "company" | "business_unit" | "team".
    #[serde(default)]
    pub org_type: String,
}

pub fn get_settings() -> Settings {
    read_json_or_default(&settings_file())
}

pub fn save_settings(s: &Settings) -> Result<()> {
    write_json(&settings_file(), s)
}

/// Read a JSON file into a type, returning the default if it does not exist.
pub fn read_json_or_default<T: serde::de::DeserializeOwned + Default>(path: &std::path::Path) -> T {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

/// Write a value as pretty JSON, creating parents and hardening perms.
pub fn write_json<T: serde::Serialize>(path: &std::path::Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let data = serde_json::to_string_pretty(value)?;
    std::fs::write(path, data).with_context(|| format!("writing {}", path.display()))?;
    harden(path);
    Ok(())
}

/// Restrict permissions to the owner (0600 files / 0700 dirs) on Unix.
fn harden(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mode = if meta.is_dir() { 0o700 } else { 0o600 };
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
        }
    }
    let _ = path;
}
