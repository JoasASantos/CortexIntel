//! External data connectors. Consistent with CortexIntel's philosophy of driving
//! the operator's own authenticated tools, database and cloud connectors shell
//! out to the standard clients (`psql`, `mysql`, `bq`, `aws`, `gsutil`) rather
//! than embedding heavy drivers. Each connector either verifies connectivity or
//! materializes rows into a temp CSV/JSON that the normal pipeline ingests.
//!
//! Secrets (passwords) are passed via environment variables, never on the
//! command line, and are never persisted with the saved connector config.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

/// A discipline-tagged source template: a pre-shaped connector the operator fills
/// with their own key/params. Consistent with the philosophy — it drives the
/// operator's *authenticated* access to a source, it doesn't embed credentials.
/// The `config` is a partial connector config; `{placeholders}` in it are the
/// params the operator supplies. Everything routes through the normal
/// test()/fetch() connectors (http/rest/postgres/datalake…).
pub fn source_catalog() -> Value {
    json!([
      // ---- OSINT (open sources; operator supplies their API key) ----
      {"id":"osint.crtsh","discipline":"OSINT","name":"crt.sh — Certificate Transparency","kind":"rest",
       "description":"Subdomains/certs for a domain from public CT logs.","method":"GET",
       "endpoint":"https://crt.sh/?q={domain}&output=json","params":["domain"],"auth":"none","records_path":""},
      {"id":"osint.rdap","discipline":"OSINT","name":"RDAP — Domain registration","kind":"rest",
       "description":"Registration/ownership for a domain (WHOIS successor).","method":"GET",
       "endpoint":"https://rdap.org/domain/{domain}","params":["domain"],"auth":"none","records_path":""},
      {"id":"osint.shodan","discipline":"OSINT","name":"Shodan — Host lookup","kind":"rest",
       "description":"Open ports/services/banners for an IP (needs a Shodan key).","method":"GET",
       "endpoint":"https://api.shodan.io/shodan/host/{ip}?key={api_key}","params":["ip","api_key"],"auth":"api_key","records_path":"data"},
      {"id":"osint.rss","discipline":"OSINT","name":"News / RSS feed","kind":"rest",
       "description":"Pull items from an RSS/Atom or JSON feed URL.","method":"GET",
       "endpoint":"{feed_url}","params":["feed_url"],"auth":"none","records_path":"items"},
      // ---- GEOINT (live geospatial feeds) ----
      {"id":"geoint.opensky","discipline":"GEOINT","name":"OpenSky — Live aircraft","kind":"rest",
       "description":"Live aircraft state vectors (lat/long/velocity) — plots on the map.","method":"GET",
       "endpoint":"https://opensky-network.org/api/states/all","params":[],"auth":"none","records_path":"states"},
      {"id":"geoint.overpass","discipline":"GEOINT","name":"Overpass — OSM features","kind":"rest",
       "description":"Query OpenStreetMap features (facilities, infrastructure) by area.","method":"POST",
       "endpoint":"https://overpass-api.de/api/interpreter","params":["query"],"body":"{query}","auth":"none","records_path":"elements"},
      {"id":"geoint.geojson","discipline":"GEOINT","name":"GeoJSON file / layer","kind":"datalake",
       "description":"Ingest a GeoJSON file (CCTV, air bases, units…) as a map layer.","provider":"local",
       "params":["uri"],"auth":"none","records_path":"features"},
      // ---- SIGINT (internal comms metadata; operator's own DB/exports) ----
      {"id":"sigint.cdr","discipline":"SIGINT","name":"CDR / comms metadata (Postgres)","kind":"postgres",
       "description":"Call/message detail records (metadata only) from your database.","params":["host","database","user","query"],"auth":"password","records_path":""},
      {"id":"sigint.netflow","discipline":"SIGINT","name":"NetFlow / comms export (file)","kind":"datalake",
       "description":"Ingest an exported NetFlow/comms-metadata CSV/JSON.","provider":"local","params":["uri"],"auth":"none","records_path":""},
      // ---- HUMINT (source/report management; operator's own case system) ----
      {"id":"humint.reports","discipline":"HUMINT","name":"Report intake (file)","kind":"datalake",
       "description":"Ingest a report/source CSV/JSON (with reliability/credibility).","provider":"local","params":["uri"],"auth":"none","records_path":""},
      {"id":"humint.casemgmt","discipline":"HUMINT","name":"Case management (Postgres)","kind":"postgres",
       "description":"Pull reports/sources from a case-management database.","params":["host","database","user","query"],"auth":"password","records_path":""},
      {"id":"humint.rest","discipline":"HUMINT","name":"Report API (REST + auth)","kind":"rest",
       "description":"Pull reports from an internal REST API (JWT/token/API-key).","method":"GET",
       "endpoint":"{endpoint}","params":["endpoint","token"],"auth":"token","records_path":""}
    ])
}

fn s<'a>(cfg: &'a Value, key: &str) -> Option<&'a str> {
    cfg.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

fn tmp(ext: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("cortex-fetch-{}.{ext}", uuid::Uuid::new_v4().simple()));
    p
}

fn have(bin: &str) -> bool {
    Command::new(bin).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Presence check for clients that don't accept `--version` (e.g. `sqlcmd`).
/// True if the binary can be spawned at all (i.e. it resolves on PATH).
fn have_help(bin: &str) -> bool {
    match Command::new(bin).arg("--version").output() {
        Ok(_) => true,
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Verify a connector's reachability. Returns a human status line.
pub fn test(kind: &str, cfg: &Value) -> Result<String> {
    match kind {
        "postgres" => {
            if !have("psql") {
                return Err(anyhow!("`psql` client not found on PATH"));
            }
            let out = pg_command(cfg, "SELECT 1")?.output()?;
            if out.status.success() {
                Ok(format!("connected to postgres {}", s(cfg, "database").unwrap_or("")))
            } else {
                Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()))
            }
        }
        "mysql" => {
            if !have("mysql") {
                return Err(anyhow!("`mysql` client not found on PATH"));
            }
            let out = my_command(cfg, "SELECT 1")?.output()?;
            if out.status.success() {
                Ok(format!("connected to mysql {}", s(cfg, "database").unwrap_or("")))
            } else {
                Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()))
            }
        }
        "bigquery" => {
            if !have("bq") {
                return Err(anyhow!("`bq` (Google Cloud SDK) not found on PATH"));
            }
            let out = Command::new("bq").arg("--version").output()?;
            Ok(format!("bq available: {}", String::from_utf8_lossy(&out.stdout).trim()))
        }
        "datalake" => {
            let provider = s(cfg, "provider").unwrap_or("local");
            match provider {
                "s3" => if have("aws") { Ok("aws CLI available".into()) } else { Err(anyhow!("`aws` CLI not found")) },
                "gcs" => if have("gsutil") { Ok("gsutil available".into()) } else { Err(anyhow!("`gsutil` not found")) },
                "local" => {
                    let uri = s(cfg, "uri").ok_or_else(|| anyhow!("missing 'uri'"))?;
                    if std::path::Path::new(uri).exists() { Ok("local path reachable".into()) } else { Err(anyhow!("path not found")) }
                }
                other => Err(anyhow!("unknown data-lake provider '{other}'")),
            }
        }
        "mssql" => {
            if !have_help("sqlcmd") {
                return Err(anyhow!("`sqlcmd` (SQL Server tools) not found on PATH"));
            }
            let out = ms_command(cfg, "SELECT 1")?.output()?;
            if out.status.success() {
                Ok(format!("connected to SQL Server {}", s(cfg, "database").unwrap_or("")))
            } else {
                Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()))
            }
        }
        "mongodb" => {
            if !have_help("mongoexport") {
                return Err(anyhow!("`mongoexport` (MongoDB Database Tools) not found on PATH"));
            }
            Ok("mongoexport available".into())
        }
        "jira" | "powerbi" | "looker" | "webhook" | "http" | "rest" | "elastic" => {
            if !have_help("curl") {
                return Err(anyhow!("`curl` not found on PATH"));
            }
            let ep_raw = s(cfg, "endpoint").ok_or_else(|| anyhow!("missing 'endpoint' URL"))?;
            let ep = fill_placeholders(ep_raw, cfg);
            let auth = if s(cfg, "jwt").is_some() { " (JWT)" } else if s(cfg, "token").is_some() { " (token)" } else if s(cfg, "api_key").is_some() { " (API key)" } else { "" };
            let missing: Vec<&str> = ep.split(['{', '}']).filter(|seg| ep.contains(&format!("{{{seg}}}"))).collect();
            if !missing.is_empty() {
                return Err(anyhow!("fill in: {}", missing.join(", ")));
            }
            Ok(format!("ready to call {} {ep}{auth}", s(cfg, "method").unwrap_or("GET")))
        }
        other => Err(anyhow!("unknown connector kind '{other}'")),
    }
}

/// Materialize a connector's data into a local temp file and return its path
/// (ready to hand to the pipeline via `sources::source_for_path`).
pub fn fetch(kind: &str, cfg: &Value) -> Result<PathBuf> {
    match kind {
        "postgres" => {
            let query = s(cfg, "query").ok_or_else(|| anyhow!("postgres connector needs a 'query'"))?;
            let copy = format!("COPY ({query}) TO STDOUT WITH CSV HEADER");
            let out = pg_command(cfg, &copy)?.output()?;
            if !out.status.success() {
                return Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            let path = tmp("csv");
            std::fs::write(&path, &out.stdout)?;
            Ok(path)
        }
        "mysql" => {
            let query = s(cfg, "query").ok_or_else(|| anyhow!("mysql connector needs a 'query'"))?;
            let out = my_command(cfg, query)?.output()?;
            if !out.status.success() {
                return Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            // mysql --batch emits TSV with a header row.
            let path = tmp("tsv");
            std::fs::write(&path, &out.stdout)?;
            Ok(path)
        }
        "bigquery" => {
            let query = s(cfg, "query").ok_or_else(|| anyhow!("bigquery connector needs a 'query'"))?;
            let out = Command::new("bq")
                .args(["query", "--nouse_legacy_sql", "--format=csv", "--max_rows=100000", query])
                .output()
                .context("running bq")?;
            if !out.status.success() {
                return Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            let path = tmp("csv");
            std::fs::write(&path, &out.stdout)?;
            Ok(path)
        }
        "datalake" => {
            let provider = s(cfg, "provider").unwrap_or("local");
            let uri = s(cfg, "uri").ok_or_else(|| anyhow!("data-lake connector needs a 'uri'"))?;
            let ext = if uri.ends_with(".json") || uri.ends_with(".jsonl") || uri.ends_with(".ndjson") {
                "json"
            } else {
                "csv"
            };
            match provider {
                "local" => Ok(PathBuf::from(uri)),
                "s3" => {
                    let path = tmp(ext);
                    let ok = Command::new("aws").args(["s3", "cp", uri, path.to_str().unwrap()]).status()?.success();
                    if ok { Ok(path) } else { Err(anyhow!("aws s3 cp failed")) }
                }
                "gcs" => {
                    let path = tmp(ext);
                    let ok = Command::new("gsutil").args(["cp", uri, path.to_str().unwrap()]).status()?.success();
                    if ok { Ok(path) } else { Err(anyhow!("gsutil cp failed")) }
                }
                other => Err(anyhow!("unknown data-lake provider '{other}'")),
            }
        }
        "mssql" => {
            let query = s(cfg, "query").ok_or_else(|| anyhow!("SQL Server connector needs a 'query'"))?;
            // -s"," + -W + -h-1 → comma-separated rows without the trailing row-count line.
            let out = ms_command(cfg, query)?.output()?;
            if !out.status.success() {
                return Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            let path = tmp("csv");
            std::fs::write(&path, &out.stdout)?;
            Ok(path)
        }
        "mongodb" => {
            let uri = s(cfg, "uri").ok_or_else(|| anyhow!("MongoDB connector needs a 'uri'"))?;
            let coll = s(cfg, "collection").ok_or_else(|| anyhow!("MongoDB connector needs a 'collection'"))?;
            let path = tmp("json");
            let mut cmd = Command::new("mongoexport");
            cmd.args(["--uri", uri, "--collection", coll, "--jsonArray", "--out", path.to_str().unwrap()]);
            if let Some(q) = s(cfg, "query") { cmd.args(["--query", q]); }
            if let Some(lim) = s(cfg, "limit") { cmd.args(["--limit", lim]); }
            let out = cmd.output().context("running mongoexport")?;
            if !out.status.success() {
                return Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            Ok(path)
        }
        "jira" | "powerbi" | "looker" | "webhook" | "http" | "rest" | "elastic" => http_fetch(cfg),
        other => Err(anyhow!("unknown connector kind '{other}'")),
    }
}

/// Generic authenticated HTTP fetch via `curl` — the custom-integration workhorse.
/// Supports method (GET/POST/…), custom headers, a request body, and several auth
/// modes (JWT, bearer token, basic, API key). Saves the response body to a temp
/// JSON file; the pipeline's JSON source finds the record array (use `records_path`
/// in the .mcp/source config for nested arrays like Elasticsearch `hits.hits`).
///
/// Config keys: endpoint (required), method, headers {obj}, body (string|json),
/// jwt | token [+ user for basic] | api_key [+ api_key_header].
/// Substitute `{key}` placeholders in a string from the config's string fields.
/// Lets a catalog template like `.../host/{ip}?key={api_key}` be filled from the
/// operator-supplied params.
fn fill_placeholders(template: &str, cfg: &Value) -> String {
    let mut out = template.to_string();
    if let Some(obj) = cfg.as_object() {
        for (k, v) in obj {
            if let Some(val) = v.as_str() {
                out = out.replace(&format!("{{{k}}}"), val);
            }
        }
    }
    out
}

fn http_fetch(cfg: &Value) -> Result<PathBuf> {
    let ep_raw = s(cfg, "endpoint").ok_or_else(|| anyhow!("connector needs an 'endpoint' URL"))?;
    let ep_filled = fill_placeholders(ep_raw, cfg);
    let ep: &str = &ep_filled;
    let path = tmp("json");
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "--fail-with-body", "--max-time", "120", "-o", path.to_str().unwrap()]);
    let method = s(cfg, "method").unwrap_or("GET").to_uppercase();
    cmd.args(["-X", &method]);
    // Auth (precedence: JWT → bearer/basic token → API key).
    if let Some(j) = s(cfg, "jwt") {
        cmd.args(["-H", &format!("Authorization: Bearer {j}")]);
    } else if let Some(tok) = s(cfg, "token") {
        if let Some(user) = s(cfg, "user") { cmd.args(["-u", &format!("{user}:{tok}")]); }
        else { cmd.args(["-H", &format!("Authorization: Bearer {tok}")]); }
    }
    if let Some(ak) = s(cfg, "api_key") {
        let hn = s(cfg, "api_key_header").unwrap_or("X-API-Key");
        cmd.args(["-H", &format!("{hn}: {ak}")]);
    }
    // Arbitrary custom headers.
    if let Some(hs) = cfg.get("headers").and_then(|v| v.as_object()) {
        for (k, v) in hs { if let Some(vv) = v.as_str() { cmd.args(["-H", &format!("{k}: {vv}")]); } }
    }
    // Request body for non-GET methods (webhook/POST integrations, ES queries…).
    if method != "GET" {
        if let Some(b) = cfg.get("body") {
            let body = if b.is_string() { fill_placeholders(b.as_str().unwrap(), cfg) } else { b.to_string() };
            cmd.args(["-H", "Content-Type: application/json", "--data-binary", &body]);
        }
    }
    cmd.args(["-H", "Accept: application/json"]);
    cmd.arg(ep);
    let out = cmd.output().context("running curl")?;
    if !out.status.success() {
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("{}", if !body.trim().is_empty() { body } else { err.to_string() }));
    }
    Ok(path)
}

fn ms_command(cfg: &Value, sql: &str) -> Result<Command> {
    let mut cmd = Command::new("sqlcmd");
    cmd.arg("-S").arg(format!("{},{}", s(cfg, "host").unwrap_or("127.0.0.1"), s(cfg, "port").unwrap_or("1433")))
        .arg("-U").arg(s(cfg, "user").ok_or_else(|| anyhow!("missing 'user'"))?)
        .arg("-d").arg(s(cfg, "database").ok_or_else(|| anyhow!("missing 'database'"))?)
        .arg("-s").arg(",")
        .arg("-W")
        .arg("-h").arg("-1")
        .arg("-Q").arg(sql);
    if let Some(pw) = s(cfg, "password") {
        cmd.env("SQLCMDPASSWORD", pw);
    }
    Ok(cmd)
}

fn pg_command(cfg: &Value, sql: &str) -> Result<Command> {
    let mut cmd = Command::new("psql");
    cmd.arg("-h").arg(s(cfg, "host").unwrap_or("127.0.0.1"))
        .arg("-p").arg(s(cfg, "port").unwrap_or("5432"))
        .arg("-U").arg(s(cfg, "user").ok_or_else(|| anyhow!("missing 'user'"))?)
        .arg("-d").arg(s(cfg, "database").ok_or_else(|| anyhow!("missing 'database'"))?)
        .arg("-v").arg("ON_ERROR_STOP=1")
        .arg("-w") // never prompt; use PGPASSWORD
        .arg("-c").arg(sql);
    if let Some(pw) = s(cfg, "password") {
        cmd.env("PGPASSWORD", pw);
    }
    Ok(cmd)
}

fn my_command(cfg: &Value, sql: &str) -> Result<Command> {
    let mut cmd = Command::new("mysql");
    cmd.arg("-h").arg(s(cfg, "host").unwrap_or("127.0.0.1"))
        .arg("-P").arg(s(cfg, "port").unwrap_or("3306"))
        .arg("-u").arg(s(cfg, "user").ok_or_else(|| anyhow!("missing 'user'"))?)
        .arg("--batch")
        .arg("--raw")
        .arg(s(cfg, "database").ok_or_else(|| anyhow!("missing 'database'"))?)
        .arg("-e").arg(sql);
    if let Some(pw) = s(cfg, "password") {
        cmd.env("MYSQL_PWD", pw);
    }
    Ok(cmd)
}
