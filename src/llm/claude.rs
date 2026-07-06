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

        let mut cmd = Command::new(&self.bin);
        cmd.arg("-p")
            .arg(&req.prompt)
            .arg("--dangerously-skip-permissions")
            .arg("--output-format")
            .arg("json")
            .arg("--append-system-prompt")
            .arg(&system);

        // Claude Code refuses `--dangerously-skip-permissions` under root/sudo
        // unless the environment declares itself externally sandboxed via
        // IS_SANDBOX=1. CortexIntel is an unattended automation harness, so we
        // set it by default (this is Claude Code's own supported escape hatch,
        // not a binary patch). Opt out with CORTEX_CLAUDE_NO_SANDBOX=1.
        if std::env::var_os("CORTEX_CLAUDE_NO_SANDBOX").is_none() {
            cmd.env("IS_SANDBOX", "1");
        }

        let model = req.model.clone().or_else(|| self.model.clone());
        if let Some(m) = &model {
            cmd.arg("--model").arg(m);
        }

        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn `{}` — is Claude Code installed?", self.bin))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.trim();
            if msg.contains("root") && msg.contains("sudo") {
                return Err(anyhow!(
                    "claude refused --dangerously-skip-permissions under root even with IS_SANDBOX=1. \
                     Ensure CORTEX_CLAUDE_NO_SANDBOX is unset, or use `--provider codex` / `--offline`. ({msg})"
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
