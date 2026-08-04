//! Google Gemini CLI backend (subscription / API key configured in the CLI).
//!
//! Invocation (non-interactive / headless print mode):
//!   gemini -p "<prompt>" [-m <model>]
//!
//! Like Codex, the agent persona (system prompt) is prepended to the prompt
//! because `gemini -p` takes a single instruction blob. The Gemini CLI prints
//! the assistant's final answer to stdout; our tolerant `extract_json` parser
//! pulls any structured value out downstream.

use super::{LlmProvider, LlmRequest, LlmResponse};
use anyhow::{anyhow, Context, Result};
use std::process::Command;

pub struct GeminiProvider {
    model: Option<String>,
    bin: String,
}

impl GeminiProvider {
    pub fn new(model: Option<String>) -> Self {
        GeminiProvider {
            model,
            bin: std::env::var("CORTEX_GEMINI_BIN").unwrap_or_else(|_| "gemini".into()),
        }
    }
}

impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut instruction = String::new();
        instruction.push_str("### ROLE\n");
        instruction.push_str(&req.system);
        if req.json_schema.is_some() {
            instruction.push_str(
                "\n\n### OUTPUT CONTRACT\nRespond with a single valid JSON value and nothing else. \
                 Do not add prose or markdown fences.",
            );
        }
        instruction.push_str("\n\n### TASK\n");
        instruction.push_str(&req.prompt);

        let model = req.model.clone().or_else(|| self.model.clone());

        let mut cmd = Command::new(&self.bin);
        cmd.arg("-p").arg(&instruction);
        if let Some(m) = &model {
            cmd.arg("-m").arg(m);
        }

        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn `{}` — is the Gemini CLI installed?", self.bin))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("gemini exited with {}: {}", output.status, stderr.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let text = stdout.trim().to_string();
        if text.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("gemini returned empty output: {}", stderr.trim()));
        }

        Ok(LlmResponse {
            text,
            provider: "gemini".into(),
            model: model.unwrap_or_else(|| "gemini".into()),
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
            Err(anyhow!("gemini --version failed"))
        }
    }
}
