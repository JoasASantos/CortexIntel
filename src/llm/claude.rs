//! Claude Code CLI backend (subscription).
//!
//! Invocation:
//!   claude -p "<prompt>" \
//!     --dangerously-skip-permissions \
//!     --output-format json \
//!     [--model <model>] \
//!     --append-system-prompt "<system>"
//!
//! `--output-format json` makes Claude Code emit a single JSON envelope whose
//! `.result` field holds the assistant's final text. We parse that so prose and
//! tool chatter never leak into the agent's structured result.

use super::{LlmProvider, LlmRequest, LlmResponse};
use anyhow::{anyhow, Context, Result};
use std::process::Command;

pub struct ClaudeProvider {
    model: Option<String>,
    bin: String,
}

impl ClaudeProvider {
    pub fn new(model: Option<String>) -> Self {
        ClaudeProvider {
            model,
            bin: std::env::var("CORTEX_CLAUDE_BIN").unwrap_or_else(|_| "claude".into()),
        }
    }
}

impl LlmProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut system = req.system.clone();
        if req.json_schema.is_some() {
            system.push_str(
                "\n\nOUTPUT CONTRACT: respond with a single valid JSON value and nothing else. \
                 No prose, no markdown fences, no commentary.",
            );
        }

        let model = req.model.clone().or_else(|| self.model.clone());

        // Claude Code's args. Built as a vector so we can run them either directly
        // or, when we're root, as the normal user (see below).
        let mut args: Vec<String> = vec![
            "-p".into(), req.prompt.clone(),
            "--dangerously-skip-permissions".into(),
            "--output-format".into(), "json".into(),
            "--append-system-prompt".into(), system,
        ];
        if let Some(m) = &model {
            args.push("--model".into());
            args.push(m.clone());
        }

        // Claude Code auth lives in the USER's home (subscription), and it refuses
        // to run under root. So by default, when CortexIntel runs as root, execute
        // `claude` AS THE NORMAL USER via `sudo -u <user>` — that uses their real
        // subscription and needs no sandbox hack. The user is CORTEX_CLAUDE_USER →
        // SUDO_USER → the console owner. Opt out with CORTEX_CLAUDE_NO_DROP=1.
        let run_as = if std::env::var_os("CORTEX_CLAUDE_NO_DROP").is_some() { None } else { run_as_user() };
        let mut cmd = if let Some(user) = &run_as {
            let mut c = Command::new("sudo");
            c.arg("-u").arg(user).arg("-H").arg(&self.bin);
            for a in &args { c.arg(a); }
            c
        } else {
            let mut c = Command::new(&self.bin);
            for a in &args { c.arg(a); }
            // Still root but not dropping (opt-out): use Claude's sandbox escape.
            if std::env::var_os("CORTEX_CLAUDE_NO_SANDBOX").is_none() {
                c.env("IS_SANDBOX", "1");
            }
            c
        };

        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn `{}` — is Claude Code installed?", self.bin))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.trim();
            if msg.contains("root") && msg.contains("sudo") {
                return Err(anyhow!(
                    "claude refused to run under root. CortexIntel runs it as the normal user by default \
                     (sudo -u); set CORTEX_CLAUDE_USER=<user> if the user couldn't be detected, or use \
                     `--provider codex` / `--offline`. ({msg})"
                ));
            }
            return Err(anyhow!("claude exited with {}: {}", output.status, msg));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (text, resolved_model) = parse_envelope(&stdout, model.as_deref());

        Ok(LlmResponse {
            text,
            provider: "claude".into(),
            model: resolved_model,
        })
    }

    fn health(&self) -> Result<String> {
        let out = Command::new(&self.bin)
            .arg("--version")
            .output()
            .with_context(|| format!("`{}` not found on PATH", self.bin))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(anyhow!("claude --version failed"))
        }
    }
}

/// True if the process is running as root (uid 0).
fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// The normal user to run Claude Code as when we're root: CORTEX_CLAUDE_USER →
/// SUDO_USER → the macOS console owner. None if we're already a normal user (or
/// can't determine one) — then Claude runs directly.
fn run_as_user() -> Option<String> {
    if let Ok(u) = std::env::var("CORTEX_CLAUDE_USER") {
        if !u.trim().is_empty() { return Some(u.trim().to_string()); }
    }
    if let Ok(u) = std::env::var("SUDO_USER") {
        if !u.trim().is_empty() && u != "root" { return Some(u.trim().to_string()); }
    }
    if is_root() {
        // macOS: the GUI console owner is the human user whose Claude auth we want.
        if let Ok(out) = Command::new("stat").args(["-f", "%Su", "/dev/console"]).output() {
            let u = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !u.is_empty() && u != "root" { return Some(u); }
        }
    }
    None
}

/// Parse Claude Code's `--output-format json` envelope. Falls back to treating
/// stdout as raw text if the envelope shape is unexpected.
fn parse_envelope(stdout: &str, requested_model: Option<&str>) -> (String, String) {
    let default_model = requested_model.unwrap_or("claude").to_string();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        let text = v
            .get("result")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| stdout.trim().to_string());
        let model = v
            .get("modelUsage")
            .and_then(|m| m.as_object())
            .and_then(|m| m.keys().next().cloned())
            .or_else(|| v.get("model").and_then(|m| m.as_str()).map(String::from))
            .unwrap_or(default_model);
        (text, model)
    } else {
        (stdout.trim().to_string(), default_model)
    }
}
