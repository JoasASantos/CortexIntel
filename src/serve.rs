//! Local HTTP server that binds the WebView frontend to the engine without
//! Tauri — one binary, no external deps. Run `cortex serve` and open the printed
//! URL in a normal browser.
//!
//! The frontend (`gui/dist`, incl. the vendored graph engine) is embedded into
//! the binary, so a release build is fully self-contained and works offline.
//! All `/api/*` routes except ping/auth/health require a bearer session token.

use crate::{api, auth, connectors, keys, plugins, projects, transforms};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};

/// Async job for long-running LLM work (ask/run/connector) so the HTTP
/// connection returns immediately and the frontend polls — no idle-timeout
/// "failed to fetch" on slow model calls.
#[derive(Clone, serde::Serialize)]
struct Job {
    status: String, // running | done | error
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn jobs() -> &'static Mutex<HashMap<String, Job>> {
    static J: OnceLock<Mutex<HashMap<String, Job>>> = OnceLock::new();
    J.get_or_init(|| Mutex::new(HashMap::new()))
}

fn start_job(kind: String, payload: serde_json::Value) -> String {
    let id = format!("job-{}", uuid::Uuid::new_v4().simple());
    jobs().lock().unwrap().insert(id.clone(), Job { status: "running".into(), result: None, error: None });
    let jid = id.clone();
    std::thread::spawn(move || {
        let res: Result<serde_json::Value> = (|| match kind.as_str() {
            "ask" => api::ask(serde_json::from_value(payload)?),
            "run" => api::run_analysis(serde_json::from_value(payload)?),
            "connector_run" => api::connector_run(serde_json::from_value(payload)?),
            "report_pdf" => api::report_pdf(payload.get("project_id").and_then(|v| v.as_str()).unwrap_or("")),
            other => Err(anyhow!("unknown job kind '{other}'")),
        })();
        let mut map = jobs().lock().unwrap();
        if let Some(j) = map.get_mut(&jid) {
            match res {
                Ok(v) => { j.status = "done".into(); j.result = Some(v); }
                Err(e) => { j.status = "error".into(); j.error = Some(e.to_string()); }
            }
        }
    });
    id
}

// Embedded frontend assets.
const INDEX_HTML: &str = include_str!("../gui/dist/index.html");
const STYLES_CSS: &str = include_str!("../gui/dist/styles.css");
const APP_JS: &str = include_str!("../gui/dist/app.js");
const V_CYTOSCAPE: &str = include_str!("../gui/dist/vendor/cytoscape.min.js");
const V_LAYOUT_BASE: &str = include_str!("../gui/dist/vendor/layout-base.js");
const V_COSE_BASE: &str = include_str!("../gui/dist/vendor/cose-base.js");
const V_FCOSE: &str = include_str!("../gui/dist/vendor/cytoscape-fcose.js");

pub fn serve(port: u16, open: bool) -> Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr)
        .map_err(|e| anyhow!("cannot bind {addr}: {e} (try another --port)"))?;
    let url = format!("http://{addr}");
    println!("CortexIntel server → {url}");
    println!("  open that URL in your browser (Chrome/Safari/Firefox). Ctrl-C to stop.");
    if open {
        let _ = open_browser(&url);
    }
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(move || {
                    if let Err(e) = handle(s) {
                        eprintln!("  · request error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("  · accept error: {e}"),
        }
    }
    Ok(())
}

struct Req {
    method: String,
    path: String,
    query: String,
    body: Vec<u8>,
    token: Option<String>,
}

fn handle(mut stream: TcpStream) -> Result<()> {
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut tmp = [0u8; 8192];
    let headers_end = loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > 1 << 20 {
            return respond(&mut stream, 431, "text/plain", b"headers too large");
        }
    };

    let header_txt = String::from_utf8_lossy(&buf[..headers_end]).into_owned();
    let mut lines = header_txt.lines();
    let req_line = lines.next().unwrap_or("");
    let mut it = req_line.split_whitespace();
    let method = it.next().unwrap_or("").to_string();
    let target = it.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    let mut token = None;
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            let k = k.trim();
            if k.eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            } else if k.eq_ignore_ascii_case("authorization") {
                let v = v.trim();
                token = v.strip_prefix("Bearer ").map(|s| s.to_string());
            }
        }
    }

    let mut body = buf[headers_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };

    let req = Req { method, path, query, body, token };
    route(&mut stream, &req)
}

fn route(stream: &mut TcpStream, req: &Req) -> Result<()> {
    let (m, p) = (req.method.as_str(), req.path.as_str());

    // --- Public: static assets ---
    match (m, p) {
        ("GET", "/") | ("GET", "/index.html") => return respond(stream, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes()),
        ("GET", "/styles.css") => return respond(stream, 200, "text/css; charset=utf-8", STYLES_CSS.as_bytes()),
        ("GET", "/app.js") => return respond(stream, 200, "application/javascript; charset=utf-8", APP_JS.as_bytes()),
        ("GET", "/vendor/cytoscape.min.js") => return respond(stream, 200, "application/javascript; charset=utf-8", V_CYTOSCAPE.as_bytes()),
        ("GET", "/vendor/layout-base.js") => return respond(stream, 200, "application/javascript; charset=utf-8", V_LAYOUT_BASE.as_bytes()),
        ("GET", "/vendor/cose-base.js") => return respond(stream, 200, "application/javascript; charset=utf-8", V_COSE_BASE.as_bytes()),
        ("GET", "/vendor/cytoscape-fcose.js") => return respond(stream, 200, "application/javascript; charset=utf-8", V_FCOSE.as_bytes()),
        ("OPTIONS", _) => return respond(stream, 204, "text/plain", b""),
        _ => {}
    }

    // --- Public: unauthenticated API ---
    match (m, p) {
        ("GET", "/api/ping") => return respond(stream, 200, "application/json; charset=utf-8", br#"{"cortex":true,"version":"0.1.0"}"#),
        ("GET", "/api/health") => return json_ok(stream, &health()),
        ("GET", "/api/auth/status") => return json_ok(stream, &serde_json::json!({ "has_accounts": auth::has_accounts() })),
        ("POST", "/api/auth/register") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            return finish(stream, auth::register(
                b.get("email").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("display_name").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("password").and_then(|v| v.as_str()).unwrap_or(""),
            ));
        }
        ("POST", "/api/auth/login") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            return finish(stream, auth::login(
                b.get("email").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("password").and_then(|v| v.as_str()).unwrap_or(""),
            ));
        }
        _ => {}
    }

    // --- Everything else requires a valid session ---
    let user = match req.token.as_deref().and_then(auth::validate) {
        Some(u) => u,
        None => return respond(stream, 401, "application/json; charset=utf-8", br#"{"error":"authentication required"}"#),
    };

    match (m, p) {
        ("POST", "/api/auth/logout") => { if let Some(t) = &req.token { auth::logout(t); } json_ok(stream, &serde_json::json!({"ok": true})) }
        ("GET", "/api/me") => json_ok(stream, &user),
        ("GET", "/api/domains") => json_ok(stream, &api::list_domains()),
        ("GET", "/api/data_types") => json_ok(stream, &api::list_data_types()),
        ("GET", "/api/agents") => json_ok(stream, &api::list_agents(&param(&req.query, "domain").unwrap_or_else(|| "generic".into()))),
        ("GET", "/api/doctor") => json_ok(stream, &api::doctor()),
        ("GET", "/api/graph") => match param(&req.query, "dir") {
            Some(dir) => finish(stream, api::load_graph(&dir)),
            None => json_err(stream, "missing dir"),
        },
        ("POST", "/api/run") => finish(stream, parse_body(&req.body).and_then(api::run_analysis)),
        ("POST", "/api/ask") => finish(stream, parse_body(&req.body).and_then(api::ask)),
        // Async jobs (used by the UI for long LLM calls)
        ("POST", "/api/jobs") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            let kind = b.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let payload = b.get("payload").cloned().unwrap_or(serde_json::json!({}));
            if !["ask", "run", "connector_run", "report_pdf"].contains(&kind.as_str()) {
                return json_err(stream, "invalid job kind");
            }
            let id = start_job(kind, payload);
            json_ok(stream, &serde_json::json!({ "job_id": id }))
        }
        ("GET", "/api/jobs/status") => match param(&req.query, "id") {
            Some(id) => match jobs().lock().unwrap().get(&id) {
                Some(j) => json_ok(stream, j),
                None => json_err(stream, "unknown job"),
            },
            None => json_err(stream, "missing id"),
        },
        ("POST", "/api/report/pdf") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, api::report_pdf(b.get("project_id").and_then(|v| v.as_str()).unwrap_or("")))
        }
        ("GET", "/api/report/download") => match param(&req.query, "path") {
            Some(path) if path.ends_with(".pdf") && path.contains("report-") => match std::fs::read(&path) {
                Ok(bytes) => respond(stream, 200, "application/pdf", &bytes),
                Err(e) => json_err(stream, &e.to_string()),
            },
            _ => json_err(stream, "invalid path"),
        },
        // Projects
        ("GET", "/api/projects") => json_ok(stream, &projects::list()),
        ("GET", "/api/projects/get") => match param(&req.query, "id") {
            Some(id) => finish(stream, projects::load(&id)),
            None => json_err(stream, "missing id"),
        },
        ("POST", "/api/projects") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, projects::create(
                b.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("domain").and_then(|v| v.as_str()).unwrap_or("generic"),
                &user.email,
                b.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("ai_instructions").and_then(|v| v.as_str()).unwrap_or(""),
            ))
        }
        ("POST", "/api/projects/delete") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, projects::delete(b.get("id").and_then(|v| v.as_str()).unwrap_or("")).map(|_| serde_json::json!({"ok":true})))
        }
        ("GET", "/api/projects/export") => match param(&req.query, "id") {
            Some(id) => match projects::export(&id) { Ok(s) => respond(stream, 200, "application/json; charset=utf-8", s.as_bytes()), Err(e) => json_err(stream, &e.to_string()) },
            None => json_err(stream, "missing id"),
        },
        ("POST", "/api/projects/import") => {
            let raw = String::from_utf8_lossy(&req.body).to_string();
            finish(stream, projects::import(&raw, &user.email))
        }
        // Connectors
        ("POST", "/api/connectors/test") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, connectors::test(
                b.get("kind").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("config").unwrap_or(&serde_json::Value::Null),
            ).map(|s| serde_json::json!({ "status": s })))
        }
        ("POST", "/api/connectors/run") => finish(stream, parse_body(&req.body).and_then(api::connector_run)),
        // Plugins
        ("GET", "/api/plugins") => json_ok(stream, &plugins::list()),
        ("POST", "/api/plugins/install") => {
            let raw = String::from_utf8_lossy(&req.body).to_string();
            finish(stream, plugins::install(&raw))
        }
        ("POST", "/api/plugins/enable") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, plugins::set_enabled(
                b.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            ).map(|_| serde_json::json!({"ok":true})))
        }
        ("POST", "/api/plugins/remove") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, plugins::remove(b.get("id").and_then(|v| v.as_str()).unwrap_or("")).map(|_| serde_json::json!({"ok":true})))
        }
        // Transforms (Maltego-style store + execution)
        ("GET", "/api/transforms/catalog") => json_ok(stream, &transforms::catalog()),
        ("GET", "/api/transforms") => json_ok(stream, &transforms::list()),
        ("POST", "/api/transforms/install") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, transforms::install_from_catalog(b.get("id").and_then(|v| v.as_str()).unwrap_or("")))
        }
        ("POST", "/api/transforms/install_manifest") => finish(stream, parse_body(&req.body).and_then(transforms::install_manifest)),
        ("POST", "/api/transforms/enable") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, transforms::set_enabled(b.get("id").and_then(|v| v.as_str()).unwrap_or(""), b.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)).map(|_| serde_json::json!({"ok":true})))
        }
        ("POST", "/api/transforms/remove") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, transforms::remove(b.get("id").and_then(|v| v.as_str()).unwrap_or("")).map(|_| serde_json::json!({"ok":true})))
        }
        ("POST", "/api/transforms/run") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, transforms::run(
                b.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                b.get("input").cloned().unwrap_or(serde_json::Value::Null),
                b.get("params").cloned().unwrap_or(serde_json::json!({})),
            ))
        }
        // Instance config (country / onboarding)
        ("GET", "/api/config") => json_ok(stream, &api::get_config()),
        ("POST", "/api/config") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, api::set_config(b.get("country").and_then(|v| v.as_str()).unwrap_or(""), b.get("onboarded").and_then(|v| v.as_bool()).unwrap_or(false)))
        }
        // File upload (browse a file from the PC → temp path for the pipeline)
        ("POST", "/api/upload") => {
            let name = param(&req.query, "name").unwrap_or_else(|| "upload.dat".into());
            finish(stream, api::save_upload(&name, &req.body).map(|p| serde_json::json!({ "path": p })))
        }
        // API keys (values never returned)
        ("GET", "/api/keys") => json_ok(stream, &keys::list_names()),
        ("POST", "/api/keys") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, keys::set(b.get("service").and_then(|v| v.as_str()).unwrap_or(""), b.get("key").and_then(|v| v.as_str()).unwrap_or("")).map(|_| serde_json::json!({"ok":true})))
        }
        ("POST", "/api/keys/delete") => {
            let b: serde_json::Value = parse_body(&req.body)?;
            finish(stream, keys::delete(b.get("service").and_then(|v| v.as_str()).unwrap_or("")).map(|_| serde_json::json!({"ok":true})))
        }
        _ => respond(stream, 404, "application/json; charset=utf-8", br#"{"error":"not found"}"#),
    }
}

/// Loading-screen payload: which modules and plugins are available.
fn health() -> serde_json::Value {
    let backends: Vec<serde_json::Value> = api::doctor()
        .into_iter()
        .map(|h| serde_json::json!({ "name": h.name, "ok": h.ok, "detail": h.detail }))
        .collect();
    let modules = [
        "ingestion", "normalization", "entity-extraction", "graph-correlation",
        "risk-prioritization", "investigation", "audit", "connectors", "ai-copilot",
    ];
    serde_json::json!({
        "cortex": true,
        "version": "0.1.0",
        "modules": modules,
        "backends": backends,
        "plugins": plugins::list(),
        "data_dir": crate::store::base_dir().display().to_string(),
        "has_accounts": auth::has_accounts(),
    })
}

fn parse_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T> {
    serde_json::from_slice(body).map_err(|e| anyhow!("bad request body: {e}"))
}

fn finish<T: serde::Serialize>(stream: &mut TcpStream, r: Result<T>) -> Result<()> {
    match r {
        Ok(v) => json_ok(stream, &v),
        Err(e) => json_err(stream, &e.to_string()),
    }
}

fn json_ok<T: serde::Serialize>(stream: &mut TcpStream, v: &T) -> Result<()> {
    let body = serde_json::to_vec(v)?;
    respond(stream, 200, "application/json; charset=utf-8", &body)
}

fn json_err(stream: &mut TcpStream, msg: &str) -> Result<()> {
    let body = serde_json::json!({ "error": msg });
    respond(stream, 400, "application/json; charset=utf-8", serde_json::to_string(&body)?.as_bytes())
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) -> Result<()> {
    let reason = match status {
        200 => "OK", 204 => "No Content", 400 => "Bad Request", 401 => "Unauthorized",
        404 => "Not Found", 431 => "Request Header Fields Too Large", _ => "OK",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(percent_decode(v));
            }
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let bytes = s.replace('+', " ");
    let bytes = bytes.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&String::from_utf8_lossy(&bytes[i + 1..i + 3]), 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![url]);
    #[cfg(target_os = "linux")]
    let cmd = ("xdg-open", vec![url]);
    #[cfg(target_os = "windows")]
    let cmd = ("cmd", vec!["/C", "start", url]);
    std::process::Command::new(cmd.0).args(cmd.1).spawn()?;
    Ok(())
}
