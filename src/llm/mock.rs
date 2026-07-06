//! Deterministic offline backend. Returns valid, minimal JSON shaped per agent
//! label so the whole pipeline can run end-to-end with no external calls and no
//! cost (CI, smoke tests, air-gapped demos). The heuristic layer in each stage
//! still does the real extraction/correlation work; the mock just stands in for
//! the LLM augmentation step.

use super::{LlmProvider, LlmRequest, LlmResponse};
use anyhow::Result;

#[derive(Default)]
pub struct MockProvider;

impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let label = req.agent_label.as_str();
        let text = if label.contains("classify") {
            r#"{"data_type":"generic","confidence":0.5,"rationale":"offline mock classification"}"#
                .to_string()
        } else if label.contains("extract") {
            r#"{"entities":[],"relationships":[]}"#.to_string()
        } else if label.contains("correlate") {
            r#"{"relationships":[]}"#.to_string()
        } else if label.contains("risk") {
            r#"{"assessments":[]}"#.to_string()
        } else if label.contains("ask") {
            r#"{"answer":"[offline mock] Connect Claude or Codex to get real AI intelligence over this graph. Structurally, look for shared hubs (same IP/device/wallet) linking otherwise separate entities — those are your strongest correlation leads.","key_points":["Shared-hub entities are correlation pivots","High-degree nodes concentrate risk"],"recommended_actions":["Switch provider to Claude/Codex for live analysis"],"entities":[],"relationships":[],"confidence":"low"}"#
                .to_string()
        } else if label.contains("investigat") {
            r#"{"summary":"offline mock — no LLM narrative generated","recommended_actions":[],"next_steps":[]}"#
                .to_string()
        } else if req.json_schema.is_some() {
            "{}".to_string()
        } else {
            "offline mock response".to_string()
        };

        Ok(LlmResponse {
            text,
            provider: "mock".into(),
            model: "offline".into(),
        })
    }

    fn health(&self) -> Result<String> {
        Ok("offline mock provider ready".into())
    }
}
