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
        other => Err(anyhow!("unknown connector kind '{other}'")),
    }
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
