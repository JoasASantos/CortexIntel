//! Generic CLI model backend — plug ANY model that runs as a command and reads a
//! prompt. Set `CORTEX_LLM_CMD` to the command (e.g. `ollama run llama3`,
//! `llm -m gpt-4o`, or a wrapper script for Gemini/DeepSeek/Azure/a local model).
//!
//! Contract: the prompt (role + task) is written to the command's STDIN and the
//! answer is read from STDOUT. If the command contains the token `{prompt}`, it is
//! substituted into the arguments instead of using stdin. This keeps CortexIntel's
//! "drive the operator's tools" philosophy while supporting many more models, with
//! no HTTP client and no embedded keys.

use super::{LlmProvider, LlmRequest, LlmResponse};
use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub struct GenericProvider {
    cmd: Option<String>,
}

impl GenericProvider {
    pub fn new() -> Self {
        GenericProvider { cmd: std::env::var("CORTEX_LLM_CMD").ok().filter(|s| !s.trim().is_empty()) }
    }
    pub fn is_configured(&self) -> bool {
        self.cmd.is_some()
    }
    fn parts(&self) -> Result<Vec<String>> {
        let raw = self.cmd.clone().ok_or_else(|| anyhow!("CORTEX_LLM_CMD not set — no custom model configured"))?;
        Ok(raw.split_whitespace().map(|s| s.to_string()).collect())
    }
}

impl LlmProvider for GenericProvider {
    fn name(&self) -> &str {
        "custom"
    }

    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let parts = self.parts()?;
        let (bin, args) = parts.split_first().ok_or_else(|| anyhow!("empty CORTEX_LLM_CMD"))?;

        let mut instruction = String::new();
        instruction.push_str(&req.system);
        if req.json_schema.is_some() {
            instruction.push_str("\n\nRespond with a single valid JSON value and nothing else — no prose, no markdown fences.");
        }
        instruction.push_str("\n\n");
        instruction.push_str(&req.prompt);

        let uses_placeholder = args.iter().any(|a| a.contains("{prompt}"));
        let mut cmd = Command::new(bin);
        for a in args {
            cmd.arg(a.replace("{prompt}", &instruction));
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        if !uses_placeholder {
            cmd.stdin(Stdio::piped());
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn custom model `{bin}` (CORTEX_LLM_CMD)"))?;
        if !uses_placeholder {
            if let Some(mut sin) = child.stdin.take() {
                let _ = sin.write_all(instruction.as_bytes());
            }
        }
        let output = child.wait_with_output().context("custom model process failed")?;
        if !output.status.success() {
            return Err(anyhow!("custom model exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr).trim()));
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err(anyhow!("custom model returned empty output"));
        }
        Ok(LlmResponse { text, provider: "custom".into(), model: bin.clone() })
    }

    fn health(&self) -> Result<String> {
        match &self.cmd {
            Some(c) => Ok(format!("configured: {c}")),
            None => Err(anyhow!("CORTEX_LLM_CMD not set")),
        }
    }
}
