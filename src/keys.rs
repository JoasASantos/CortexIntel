//! API-key vault for transforms that call public services (Shodan, VirusTotal,
//! WHOIS, company registries, …). Keys are stored in `~/.cortexintel/keys.json`
//! (0600) and only ever leave the process into a transform's stdin/env — the
//! frontend can list which services have a key, never the values.

use crate::store;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Serialize, Deserialize)]
struct KeyDb {
    keys: BTreeMap<String, String>,
}

fn path() -> std::path::PathBuf {
    store::base_dir().join("keys.json")
}
fn load() -> KeyDb {
    store::read_json_or_default(&path())
}
fn save(db: &KeyDb) -> Result<()> {
    store::write_json(&path(), db)
}

/// Service names that currently have a stored key (values never exposed).
pub fn list_names() -> Vec<String> {
    load().keys.keys().cloned().collect()
}

pub fn set(service: &str, key: &str) -> Result<()> {
    let service = service.trim();
    if service.is_empty() {
        return Err(anyhow!("service name required"));
    }
    let mut db = load();
    if key.trim().is_empty() {
        db.keys.remove(service);
    } else {
        db.keys.insert(service.to_string(), key.trim().to_string());
    }
    save(&db)
}

pub fn get(service: &str) -> Option<String> {
    load().keys.get(service).cloned()
}

pub fn delete(service: &str) -> Result<()> {
    let mut db = load();
    db.keys.remove(service);
    save(&db)
}
