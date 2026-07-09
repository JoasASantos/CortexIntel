//! Data-source connectors. CortexIntel keeps no local database: sources are read
//! into an in-memory batch of [`Record`]s (flat key/value maps), processed, and
//! written back out as JSON. Supported connectors:
//!
//!   * CSV  — one record per row, headers as keys.
//!   * JSON — an array of objects, or an object with a top-level array field.
//!   * MCP  — a manifest describing a Model Context Protocol server/tool to pull
//!            from. Since MCP calls are performed by the orchestrating agent
//!            runtime (Claude/Codex have MCP access), this connector emits a
//!            fetch *plan* that the LLM layer executes, rather than opening a
//!            socket itself.

use crate::config::DataType;
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single normalized input row.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Record {
    /// Flat field map (nested JSON is flattened with dotted keys).
    pub fields: IndexMap<String, String>,
    /// Where the record came from (file path, connector id, mcp tool…).
    pub origin: String,
    /// Row/index within the origin, for provenance.
    pub index: usize,
}

impl Record {
    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    /// Case-insensitive lookup across a set of candidate keys.
    pub fn get_any(&self, keys: &[&str]) -> Option<&str> {
        for want in keys {
            for (k, v) in &self.fields {
                if k.eq_ignore_ascii_case(want) {
                    return Some(v.as_str());
                }
            }
        }
        None
    }

    /// Concatenate all field values into one searchable text blob.
    pub fn blob(&self) -> String {
        self.fields
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A batch of records tagged with a (possibly unknown) data type.
#[derive(Debug, Clone)]
pub struct RecordBatch {
    pub records: Vec<Record>,
    pub declared_type: Option<DataType>,
    #[allow(dead_code)]
    pub origin: String,
}

/// Every connector produces a [`RecordBatch`].
pub trait DataSource {
    fn load(&self) -> Result<RecordBatch>;
    fn describe(&self) -> String;
}

/// Read a file and decode it to UTF-8, tolerating non-UTF-8 encodings common
/// across regions (Latin-1/ISO-8859-1, Windows-1252, UTF-16, etc.). Valid UTF-8
/// is used as-is; otherwise the encoding is detected and transcoded so accented
/// Latin, Cyrillic, Greek and similar data ingest cleanly instead of mojibake.
pub fn read_text_decoded(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if let Ok(s) = std::str::from_utf8(&bytes) {
        return Ok(strip_bom(s).to_string());
    }
    let mut det = chardetng::EncodingDetector::new();
    det.feed(&bytes, true);
    let enc = det.guess(None, true);
    let (cow, _, _) = enc.decode(&bytes);
    Ok(strip_bom(&cow).to_string())
}

fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

/// Autodetect a connector from a path's extension.
pub fn source_for_path(path: &Path, declared: Option<DataType>) -> Result<Box<dyn DataSource>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "csv" | "tsv" => Ok(Box::new(CsvSource {
            path: path.to_path_buf(),
            declared,
            delimiter: if ext == "tsv" { b'\t' } else { b',' },
        })),
        "json" | "ndjson" | "jsonl" | "geojson" => Ok(Box::new(JsonSource {
            path: path.to_path_buf(),
            declared,
            lines: ext == "ndjson" || ext == "jsonl",
        })),
        "yaml" | "yml" | "mcp" | "toml" => Ok(Box::new(McpSource {
            manifest: path.to_path_buf(),
            declared,
        })),
        other => Err(anyhow!(
            "unsupported source extension '.{other}' (use csv/tsv/json/jsonl/ndjson or an .mcp manifest)"
        )),
    }
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

pub struct CsvSource {
    pub path: std::path::PathBuf,
    pub declared: Option<DataType>,
    pub delimiter: u8,
}

impl DataSource for CsvSource {
    fn load(&self) -> Result<RecordBatch> {
        let text = read_text_decoded(&self.path)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .flexible(true)
            .from_reader(text.as_bytes());
        let headers = rdr
            .headers()
            .context("reading CSV headers")?
            .iter()
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();
        let origin = self.path.display().to_string();
        let mut records = Vec::new();
        for (i, row) in rdr.records().enumerate() {
            let row = row.with_context(|| format!("reading CSV row {i}"))?;
            let mut fields = IndexMap::new();
            for (j, cell) in row.iter().enumerate() {
                let key = headers
                    .get(j)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{j}"));
                let cell = cell.trim();
                if !cell.is_empty() {
                    fields.insert(key, cell.to_string());
                }
            }
            records.push(Record {
                fields,
                origin: origin.clone(),
                index: i,
            });
        }
        Ok(RecordBatch {
            records,
            declared_type: self.declared,
            origin,
        })
    }

    fn describe(&self) -> String {
        format!("csv:{}", self.path.display())
    }
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

pub struct JsonSource {
    pub path: std::path::PathBuf,
    pub declared: Option<DataType>,
    pub lines: bool,
}

impl DataSource for JsonSource {
    fn load(&self) -> Result<RecordBatch> {
        let raw = read_text_decoded(&self.path)?;
        let origin = self.path.display().to_string();
        let values: Vec<serde_json::Value> = if self.lines {
            raw.lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()
                .context("parsing NDJSON")?
        } else {
            let v: serde_json::Value = serde_json::from_str(&raw).context("parsing JSON")?;
            match v {
                serde_json::Value::Array(a) => a,
                serde_json::Value::Object(ref map) => {
                    // Find the first array-valued field, else treat the object as one record.
                    map.values()
                        .find_map(|x| x.as_array().cloned())
                        .unwrap_or_else(|| vec![v.clone()])
                }
                other => vec![other],
            }
        };

        let mut records = Vec::new();
        for (i, val) in values.into_iter().enumerate() {
            let mut fields = IndexMap::new();
            flatten_json("", &val, &mut fields);
            records.push(Record {
                fields,
                origin: origin.clone(),
                index: i,
            });
        }
        Ok(RecordBatch {
            records,
            declared_type: self.declared,
            origin,
        })
    }

    fn describe(&self) -> String {
        format!("json:{}", self.path.display())
    }
}

/// Flatten nested JSON into dotted keys; arrays get indexed keys.
fn flatten_json(prefix: &str, val: &serde_json::Value, out: &mut IndexMap<String, String>) {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_json(&key, v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = format!("{prefix}.{i}");
                flatten_json(&key, v, out);
            }
        }
        serde_json::Value::Null => {}
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// MCP (Model Context Protocol) manifest connector
// ---------------------------------------------------------------------------

/// An MCP connector doesn't open a socket itself. It reads a small manifest that
/// declares which MCP server + tool to call and with what arguments, and turns
/// it into a fetch *plan*. The LLM layer (Claude/Codex, which have live MCP
/// access) executes the plan and the returned rows are fed back as records.
pub struct McpSource {
    pub manifest: std::path::PathBuf,
    pub declared: Option<DataType>,
}

/// The shape of an .mcp manifest (JSON for simplicity; also accepts inline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpManifest {
    /// MCP server name as registered in the operator's Claude/Codex config.
    pub server: String,
    /// Tool to invoke on that server.
    pub tool: String,
    /// Arguments passed to the tool.
    #[serde(default)]
    pub arguments: serde_json::Value,
    /// Optional JSONPath-ish field whose array holds the records.
    #[serde(default)]
    pub records_path: Option<String>,
    /// Human description.
    #[serde(default)]
    pub description: Option<String>,
}

impl McpSource {
    /// Parse and return the manifest so the pipeline can hand it to the agent
    /// runtime as a fetch plan.
    pub fn manifest(&self) -> Result<McpManifest> {
        let raw = std::fs::read_to_string(&self.manifest)
            .with_context(|| format!("cannot read MCP manifest {}", self.manifest.display()))?;
        // Accept JSON directly.
        serde_json::from_str::<McpManifest>(&raw)
            .with_context(|| "MCP manifest must be JSON matching {server, tool, arguments, records_path}".to_string())
    }
}

impl DataSource for McpSource {
    fn load(&self) -> Result<RecordBatch> {
        // We cannot synchronously open the MCP transport from here; instead we
        // surface the plan as a single provenance record. The pipeline detects
        // McpSource and routes the manifest through the agent runtime.
        let m = self.manifest()?;
        let mut fields = IndexMap::new();
        fields.insert("mcp.server".into(), m.server.clone());
        fields.insert("mcp.tool".into(), m.tool.clone());
        fields.insert("mcp.arguments".into(), m.arguments.to_string());
        if let Some(rp) = &m.records_path {
            fields.insert("mcp.records_path".into(), rp.clone());
        }
        let origin = format!("mcp:{}/{}", m.server, m.tool);
        Ok(RecordBatch {
            records: vec![Record {
                fields,
                origin: origin.clone(),
                index: 0,
            }],
            declared_type: self.declared,
            origin,
        })
    }

    fn describe(&self) -> String {
        format!("mcp-manifest:{}", self.manifest.display())
    }
}
