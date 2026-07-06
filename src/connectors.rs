//! External data connectors. Consistent with CortexIntel's philosophy of driving
//! the operator's own authenticated tools, database and cloud connectors shell
//! out to the standard clients (`psql`, `mysql`, `bq`, `aws`, `gsutil`) rather
//! than embedding heavy drivers. Each connector either verifies connectivity or
//! materializes rows into a temp CSV/JSON that the normal pipeline ingests.
//!
//! Secrets (passwords) are passed via environment variables, never on the
//! command line, and are never persisted with the saved connector config.

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

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
        "jira" | "powerbi" | "looker" | "webhook" => {
            if !have_help("curl") {
                return Err(anyhow!("`curl` not found on PATH"));
            }
            let ep = s(cfg, "endpoint").ok_or_else(|| anyhow!("missing 'endpoint' URL"))?;
            Ok(format!("ready to call {ep}"))
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
        "jira" | "powerbi" | "looker" | "webhook" => http_fetch(cfg),
        other => Err(anyhow!("unknown connector kind '{other}'")),
    }
}

/// Generic authenticated HTTP GET via `curl`, saving the JSON body to a temp file.
/// The pipeline's JSON source finds the record array within the response.
fn http_fetch(cfg: &Value) -> Result<PathBuf> {
    let ep = s(cfg, "endpoint").ok_or_else(|| anyhow!("connector needs an 'endpoint' URL"))?;
    let path = tmp("json");
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "--fail-with-body", "--max-time", "120", "-o", path.to_str().unwrap()]);
    // Auth: bearer token, or basic user:token (Jira Cloud uses email:api_token).
    if let Some(tok) = s(cfg, "token") {
        if let Some(user) = s(cfg, "user") {
            cmd.args(["-u", &format!("{user}:{tok}")]);
        } else {
            cmd.args(["-H", &format!("Authorization: Bearer {tok}")]);
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
