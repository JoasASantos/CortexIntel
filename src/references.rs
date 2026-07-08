//! Reference sources: integrated feeds of known values (file hashes today) that
//! entities are matched against to generate intelligence — e.g. a known-malware
//! hash set, a known-CSAM hash list (Project VIC / PhotoDNA-style), or a
//! watchlist. A hash is a one-way fingerprint, so matching by hash never requires
//! holding the file itself.
//!
//! Sources are loaded from the path in the `CORTEX_REFS` env var: either a single
//! JSON file or a directory of `*.json` files. Each file is one group or an array
//! of groups:
//! ```json
//! { "source": "Known-CSAM (demo)", "category": "known_csam_reference",
//!   "severity": "critical", "kind": "hash", "values": ["<hash>", "..."] }
//! ```
//! Absent env / missing path = no reference sources (silent, offline no-op).

use serde::Deserialize;
use std::collections::HashMap;

/// A single matched reference record (what the hit is and where it came from).
#[derive(Debug, Clone)]
pub struct RefHit {
    pub source: String,
    pub category: String,
    pub severity: String,
}

#[derive(Debug, Deserialize)]
struct RefGroup {
    source: String,
    #[serde(default)]
    category: String,
    #[serde(default = "default_severity")]
    severity: String,
    /// "hash" (exact file-hash match, default) or "perceptual" (near-duplicate
    /// image match by Hamming distance — catches recompressed/altered copies).
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    values: Vec<String>,
}

fn default_kind() -> String {
    "hash".into()
}

/// A perceptual reference value: its bits + metadata, matched by similarity.
#[derive(Debug, Clone)]
struct PerceptualRef {
    bits: Vec<u8>,
    hit: RefHit,
}

/// A near-duplicate perceptual match: the reference + how similar (0..1) it is.
#[derive(Debug, Clone)]
pub struct PerceptualMatch {
    pub hit: RefHit,
    pub similarity: f32,
    pub distance: u32,
    pub bits: u32,
}

fn default_severity() -> String {
    "high".into()
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RefFile {
    One(RefGroup),
    Many(Vec<RefGroup>),
}

/// An in-memory index of reference values → hit metadata. Lowercased keys.
#[derive(Debug, Default)]
pub struct RefSets {
    by_value: HashMap<String, RefHit>,
    perceptual: Vec<PerceptualRef>,
    pub source_count: usize,
}

impl RefSets {
    pub fn len(&self) -> usize {
        self.by_value.len() + self.perceptual.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_value.is_empty() && self.perceptual.is_empty()
    }
    /// Exact lookup (case-insensitive). Returns the reference hit if known.
    pub fn lookup(&self, value: &str) -> Option<&RefHit> {
        self.by_value.get(&value.trim().to_lowercase())
    }

    /// Nearest perceptual match within `max_distance` bits, or None. Compares
    /// only against references of the same bit-length. Picks the closest.
    pub fn lookup_perceptual(&self, phash: &str, max_distance: u32) -> Option<PerceptualMatch> {
        let bits = match hex_to_bytes(phash) {
            Some(b) if !b.is_empty() => b,
            _ => return None,
        };
        let nbits = (bits.len() * 8) as u32;
        let mut best: Option<PerceptualMatch> = None;
        for p in &self.perceptual {
            if p.bits.len() != bits.len() {
                continue; // only compare equal bit-lengths
            }
            let dist: u32 = bits.iter().zip(&p.bits).map(|(a, b)| (a ^ b).count_ones()).sum();
            if dist <= max_distance {
                let similarity = 1.0 - (dist as f32 / nbits as f32);
                if best.as_ref().map(|m| dist < m.distance).unwrap_or(true) {
                    best = Some(PerceptualMatch { hit: p.hit.clone(), similarity, distance: dist, bits: nbits });
                }
            }
        }
        best
    }

    fn ingest(&mut self, groups: Vec<RefGroup>) {
        for g in groups {
            self.source_count += 1;
            let hit = |g: &RefGroup| RefHit { source: g.source.clone(), category: g.category.clone(), severity: g.severity.clone() };
            let perceptual = g.kind.eq_ignore_ascii_case("perceptual");
            for v in &g.values {
                let key = v.trim().to_lowercase();
                if key.is_empty() {
                    continue;
                }
                if perceptual {
                    if let Some(bits) = hex_to_bytes(&key) {
                        self.perceptual.push(PerceptualRef { bits, hit: hit(&g) });
                    }
                } else {
                    self.by_value.insert(key, hit(&g));
                }
            }
        }
    }
}

/// Parse an even-length hex string into bytes; None if not clean hex.
fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    let s = s.trim().trim_start_matches("0x");
    if s.is_empty() || s.len() % 2 != 0 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Load reference sources from `CORTEX_REFS` (a JSON file or a directory of them).
/// Returns an empty set when unset or unreadable — reference matching is optional.
pub fn load() -> RefSets {
    let mut sets = RefSets::default();
    let Ok(path) = std::env::var("CORTEX_REFS") else {
        return sets;
    };
    let p = std::path::Path::new(&path);
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if p.is_dir() {
        if let Ok(rd) = std::fs::read_dir(p) {
            for entry in rd.flatten() {
                let ep = entry.path();
                if ep.extension().map(|e| e == "json").unwrap_or(false) {
                    files.push(ep);
                }
            }
        }
    } else if p.is_file() {
        files.push(p.to_path_buf());
    }
    files.sort();
    for f in files {
        if let Ok(text) = std::fs::read_to_string(&f) {
            match serde_json::from_str::<RefFile>(&text) {
                Ok(RefFile::One(g)) => sets.ingest(vec![g]),
                Ok(RefFile::Many(gs)) => sets.ingest(gs),
                Err(_) => { /* skip malformed file, keep going */ }
            }
        }
    }
    sets
}
