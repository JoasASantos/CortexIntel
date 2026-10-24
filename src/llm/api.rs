//! OpenAI-compatible HTTP API backend (Kimi / Moonshot, Qwen / DashScope,
//! DeepSeek, OpenRouter, Ollama-serve, vLLM, …) configured purely from the
//! environment / `.env`:
//!
//!   CORTEX_API_<NAME>_KEY    = sk-…                     (required)
//!   CORTEX_API_<NAME>_URL    = https://api.host/v1      (base URL; sensible defaults for kimi/qwen/deepseek)
//!   CORTEX_API_<NAME>_MODEL  = model id override        (optional)
//!
//! e.g. `CORTEX_API_DEEPSEEK_KEY=…`, `CORTEX_API_KIMI_KEY=…`, `CORTEX_API_QWEN_KEY=…`.
//! Any other NAME works too (`CORTEX_API_OPENROUTER_KEY` + `_URL`) — models are
//! then addressed as provider `api:<name>` in the catalog / `CORTEX_MODELS_JSON`.
//!
//! No HTTP crate: the request is issued through the operator's `curl`, keeping
//! the "drive the operator's tools" philosophy and a zero-dependency build.

use super::{LlmProvider, LlmRequest, LlmResponse};
use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub struct ApiProvider {
    name: String,
}

fn env_key(name: &str, suffix: &str) -> Option<String> {
    let k = format!("CORTEX_API_{}_{}", name.to_uppercase().replace('-', "_"), suffix);
    std::env::var(k).ok().filter(|s| !s.trim().is_empty())
}

fn default_url(name: &str) -> Option<&'static str> {
    match name {
        "kimi" | "moonshot" => Some("https://api.moonshot.ai/v1"),
        "qwen" | "dashscope" => Some("https://dashscope-intl.aliyuncs.com/compatible-mode/v1"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "openrouter" => Some("https://openrouter.ai/api/v1"),
        "openai" => Some("https://api.openai.com/v1"),
        "groq" => Some("https://api.groq.com/openai/v1"),
        "mistral" => Some("https://api.mistral.ai/v1"),
        "ollama" => Some("http://127.0.0.1:11434/v1"),
        _ => None,
    }
}

pub fn is_configured(name: &str) -> bool {
    // Ollama-style local servers need no key; everything else does.
    env_key(name, "KEY").is_some() || (name == "ollama" && env_key(name, "URL").is_some())
}

impl ApiProvider {
    pub fn new(name: &str) -> Self {
        ApiProvider { name: name.to_string() }
    }

    fn base_url(&self) -> Result<String> {
        env_key(&self.name, "URL")
            .or_else(|| default_url(&self.name).map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("CORTEX_API_{}_URL not set", self.name.to_uppercase()))
    }
}

impl LlmProvider for ApiProvider {
    fn name(&self) -> &str {
        "api"
    }

    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let key = env_key(&self.name, "KEY").unwrap_or_default();
        let url = format!("{}/chat/completions", self.base_url()?.trim_end_matches('/'));
        let model = req.model.clone().or_else(|| env_key(&self.name, "MODEL")).unwrap_or_else(|| self.name.clone());
        let mut system = req.system.clone();
        if req.json_schema.is_some() {
            system.push_str("\n\nRespond with a single valid JSON value and nothing else — no prose, no markdown fences.");
        }
        let mut body = serde_json::json!({
            "model": model,
            "messages": [ {"role":"system","content":system}, {"role":"user","content":req.prompt} ],
            "temperature": 0.2,
        });
        if req.json_schema.is_some() && !self.name.contains("ollama") {
            body["response_format"] = serde_json::json!({"type":"json_object"});
        }
        let curl = std::env::var("CORTEX_CURL_BIN").unwrap_or_else(|_| "curl".into());
        let mut cmd = Command::new(&curl);
        cmd.arg("-sS").arg("--max-time").arg(std::env::var("CORTEX_API_TIMEOUT").unwrap_or_else(|_| "180".into()))
            .arg("-X").arg("POST").arg(&url)
            .arg("-H").arg("Content-Type: application/json");
        if !key.is_empty() {
            cmd.arg("-H").arg(format!("Authorization: Bearer {key}"));
        }
        cmd.arg("--data-binary").arg("@-").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().with_context(|| format!("failed to spawn `{curl}` for api:{}", self.name))?;
        if let Some(mut sin) = child.stdin.take() {
            let _ = sin.write_all(body.to_string().as_bytes());
        }
        let out = child.wait_with_output().context("curl failed")?;
        if !out.status.success() {
            return Err(anyhow!("api:{} curl exited {}: {}", self.name, out.status, String::from_utf8_lossy(&out.stderr).trim()));
        }
        let v: serde_json::Value = serde_json::from_slice(&out.stdout)
            .with_context(|| format!("api:{} returned non-JSON: {}", self.name, String::from_utf8_lossy(&out.stdout).chars().take(200).collect::<String>()))?;
        if let Some(err) = v.get("error") {
            return Err(anyhow!("api:{} error: {}", self.name, err.get("message").and_then(|m| m.as_str()).unwrap_or(&err.to_string())));
        }
        let text = v.pointer("/choices/0/message/content").and_then(|c| c.as_str()).map(|s| s.to_string())
            .or_else(|| v.pointer("/choices/0/text").and_then(|c| c.as_str()).map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("api:{} reply had no choices[0].message.content", self.name))?;
        if text.trim().is_empty() {
            return Err(anyhow!("api:{} returned empty content", self.name));
        }
        Ok(LlmResponse { text, provider: format!("api:{}", self.name), model })
    }

    fn health(&self) -> Result<String> {
        if !is_configured(&self.name) {
            return Err(anyhow!("CORTEX_API_{}_KEY not set", self.name.to_uppercase()));
        }
        Ok(format!("configured: {}", self.base_url().unwrap_or_default()))
    }
}

/// Load a `.env` file (project dir, then data dir) into the process environment
/// without overriding variables that are already set. Called once at startup.
pub fn load_dotenv() {
    let mut candidates = vec![std::path::PathBuf::from(".env")];
    if let Ok(cwd) = std::env::current_dir() { candidates.push(cwd.join(".env")); }
    candidates.push(crate::store::base_dir().join(".env"));
    if let Some(h) = dirs::home_dir() { candidates.push(h.join(".cortexintel").join(".env")); }
    let mut seen = std::collections::HashSet::new();
    for p in candidates {
        let Ok(canon) = p.canonicalize() else { continue };
        if !seen.insert(canon.clone()) { continue; }
        let Ok(text) = std::fs::read_to_string(&canon) else { continue };
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') { continue; }
            let l = l.strip_prefix("export ").unwrap_or(l);
            if let Some((k, v)) = l.split_once('=') {
                let k = k.trim();
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !k.is_empty() && std::env::var(k).is_err() {
                    std::env::set_var(k, v);
                }
            }
        }
    }
}
