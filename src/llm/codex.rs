//! ChatGPT Codex CLI backend.
//!
//! Invocation:
//!   codex exec "<prompt>" \
//!     --skip-git-repo-check \
//!     --sandbox read-only \
//!     [--model <model>] \
//!     [--output-schema <file>]
//!
//! The agent persona (system prompt) is prepended to the prompt because
//! `codex exec` takes a single instruction blob. `--sandbox read-only` keeps the
//! Codex agent from mutating the operator's disk while it reasons over the data
//! payload we hand it.

use super::{LlmProvider, LlmRequest, LlmResponse};
use anyhow::{anyhow, Context, Result};
use std::process::Command;

pub struct CodexProvider {
    model: Option<String>,
    bin: String,
}

impl CodexProvider {
    pub fn new(model: Option<String>) -> Self {
        CodexProvider {
            model,
            bin: std::env::var("CORTEX_CODEX_BIN").unwrap_or_else(|_| "codex".into()),
        }
    }
}

impl LlmProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut instruction = String::new();
        instruction.push_str("### ROLE\n");
        instruction.push_str(&req.system);
        if req.json_schema.is_some() {
            instruction.push_str(
                "\n\n### OUTPUT CONTRACT\nRespond with a single valid JSON value and nothing else. \
                 Do not run shell commands. Do not add prose or markdown fences.",
            );
        }
        instruction.push_str("\n\n### TASK\n");
        instruction.push_str(&req.prompt);

        // NOTE: we intentionally do NOT pass `--output-schema`. Codex forwards it
        // to OpenAI's structured-output API, which rejects loose schemas (it
        // demands strict `additionalProperties:false` with every property listed
        // as required). The JSON contract in the instruction plus our tolerant
        // `extract_json` parser are more robust across models.
        let mut cmd = Command::new(&self.bin);
        cmd.arg("exec")
            .arg("--skip-git-repo-check")
            .arg("--sandbox")
            .arg("read-only")
            .arg("--color")
            .arg("never");

        let model = req.model.clone().or_else(|| self.model.clone());
        if let Some(m) = &model {
            cmd.arg("--model").arg(m);
        }
        cmd.arg(&instruction);

        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn `{}` — is Codex installed?", self.bin))?;

        // Codex can exit 0 even when the underlying API returns an error, so we
        // check both the exit status and the stdout body for error envelopes.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "codex exited with {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(err) = detect_api_error(&stdout) {
            return Err(anyhow!("codex API error: {err}"));
        }
        let text = extract_final_message(&stdout);

        Ok(LlmResponse {
            text,
            provider: "codex".into(),
            model: model.unwrap_or_else(|| "codex".into()),
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
            Err(anyhow!("codex --version failed"))
        }
    }
}

/// Detect an OpenAI/Codex error envelope printed to stdout (which can happen
/// even on a 0 exit code) so the router can fall back to another provider.
fn detect_api_error(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let l = line.trim();
        if l.starts_with("ERROR:") || l.contains("invalid_request_error") || l.contains("\"status\": 4") {
            return Some(l.trim_start_matches("ERROR:").trim().to_string());
        }
    }
    None
}

/// `codex exec` (text mode) prints a human-readable transcript. The final
/// assistant message is what follows the last "codex" section marker; if we
/// can't locate markers we return the whole stdout trimmed. The JSON extractor
/// downstream then pulls the structured value out.
fn extract_final_message(stdout: &str) -> String {
    // Codex delimits turns with lines like "codex" / "user" or "[timestamp] codex".
    // Take everything after the last standalone "codex" marker line if present.
    let lines: Vec<&str> = stdout.lines().collect();
    let mut last_marker: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let l = line.trim();
        if l == "codex" || l.ends_with("] codex") || l == "assistant" {
            last_marker = Some(i);
        }
    }
    if let Some(m) = last_marker {
        // Stop before Codex's own footer (e.g. a "tokens used" summary block).
        let mut tail_lines: Vec<&str> = Vec::new();
        for line in &lines[m + 1..] {
            let l = line.trim();
            if l == "tokens used" || l.starts_with("tokens used") {
                break;
            }
            tail_lines.push(line);
        }
        let tail = tail_lines.join("\n");
        let tail = tail.trim();
        if !tail.is_empty() {
            return tail.to_string();
        }
    }
    stdout.trim().to_string()
}
