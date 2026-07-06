//! The agent layer. Each pipeline stage is driven by a specialized agent with
//! its own persona (system prompt), task template, and JSON output contract.
//! Agents are selected by the active vertical (domain) and the classified data
//! type. The pipeline invokes them through the LLM router.

use crate::config::{DataType, Domain};
use crate::llm::{Complexity, LlmRequest};
use crate::prompts;
use serde_json::json;

/// The stages of the intelligence pipeline (the flow from the brief:
/// ingestion → normalization/dedup → entities → graph → risk → investigation →
/// audit/retention/disposal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Classification,
    Extraction,
    Correlation,
    Risk,
    Investigation,
    Audit,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Classification => "classification",
            Stage::Extraction => "extraction",
            Stage::Correlation => "correlation",
            Stage::Risk => "risk",
            Stage::Investigation => "investigation",
            Stage::Audit => "audit",
        }
    }
}

/// A human-facing description of an agent for `cortex agents`.
#[derive(Debug, Clone)]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub stage: Stage,
    pub mission: String,
    pub domain_specialized: bool,
}

/// List the agents active for a given vertical.
pub fn catalog(domain: Domain) -> Vec<AgentCard> {
    let d = domain.slug();
    let mut cards = vec![
        AgentCard {
            id: format!("{d}.classifier"),
            name: "Data Classifier".into(),
            stage: Stage::Classification,
            mission: "Classify incoming records into a data type to route the right extractor.".into(),
            domain_specialized: false,
        },
        AgentCard {
            id: format!("{d}.extractor"),
            name: "Entity & Relationship Extractor".into(),
            stage: Stage::Extraction,
            mission: "Turn records into normalized ontology entities and typed links.".into(),
            domain_specialized: true,
        },
        AgentCard {
            id: format!("{d}.correlator"),
            name: "Graph Correlation Analyst".into(),
            stage: Stage::Correlation,
            mission: "Discover cross-entity relationships and clusters in the graph.".into(),
            domain_specialized: true,
        },
        AgentCard {
            id: format!("{d}.risk"),
            name: "Risk Prioritization Agent".into(),
            stage: Stage::Risk,
            mission: "Produce explainable risk scores and prioritized next actions.".into(),
            domain_specialized: true,
        },
        AgentCard {
            id: format!("{d}.investigator"),
            name: "Lead Investigation Agent".into(),
            stage: Stage::Investigation,
            mission: "Synthesize findings, leads and protective/evidence steps.".into(),
            domain_specialized: true,
        },
        AgentCard {
            id: format!("{d}.auditor"),
            name: "Audit & Governance Agent".into(),
            stage: Stage::Audit,
            mission: "Review the run for authorization, sensitivity and retention.".into(),
            domain_specialized: false,
        },
    ];

    // Vertical-specific specialist notes appended to the extractor/investigator.
    if let Some(extra) = specialist_note(domain) {
        cards.push(AgentCard {
            id: format!("{d}.specialist"),
            name: specialist_name(domain).into(),
            stage: Stage::Investigation,
            mission: extra.into(),
            domain_specialized: true,
        });
    }
    cards
}

fn specialist_name(domain: Domain) -> &'static str {
    match domain {
        Domain::ChildProtection => "Victim-Protection Specialist",
        Domain::Cybersecurity => "Threat-Intel Specialist",
        Domain::Fraud => "Financial-Crime Specialist",
        Domain::Health => "Clinical-Safety Specialist",
        Domain::Commerce => "Commercial-Decisioning Specialist",
        Domain::Logistics => "Supply-Chain Specialist",
        Domain::Military => "Defense-Intelligence Specialist",
        Domain::Finance => "Financial-Crime Specialist",
        _ => "Domain Specialist",
    }
}

fn specialist_note(domain: Domain) -> Option<&'static str> {
    match domain {
        Domain::ChildProtection => Some("Focus on imminent child risk, victim identification support and takedown priority; never expose sensitive media."),
        Domain::Cybersecurity => Some("Map infrastructure, actors and TTPs; recommend containment and hunting leads."),
        Domain::Fraud => Some("Trace money flows across accounts/wallets; quantify exposure and mule networks."),
        Domain::Health => Some("Correlate safety signals while enforcing patient-privacy minimization."),
        Domain::Commerce => Some("Surface commercial risk/opportunity from customer and order signals."),
        Domain::Logistics => Some("Model disruptions and dependencies; recommend resilient routing."),
        Domain::Military => Some("Correlate actors, units, infrastructure and movements; human-reviewed assessments only, never targeting."),
        Domain::Finance => Some("Trace flows across accounts/counterparties; quantify exposure."),
        Domain::Kyc => Some("Correlate a person's connected records/documents; assess identity plausibility (country-aware). Decision-support only, respect LGPD/GDPR."),
        Domain::Government | Domain::Legal | Domain::Insurance | Domain::Telecom | Domain::Energy | Domain::Manufacturing | Domain::RealEstate | Domain::Education | Domain::Nonprofit => Some("Correlate the vertical's core records into prioritized, auditable, human-reviewed intelligence."),
        Domain::Generic => None,
    }
}

// ---------------------------------------------------------------------------
// Request builders — the pipeline calls these to talk to the router.
// ---------------------------------------------------------------------------

pub fn classify_request(domain: Domain, sample: &str) -> LlmRequest {
    // Cheap routing decision → simplest tier (Codex).
    LlmRequest::new(prompts::classifier_system(domain), prompts::classify_task(sample))
        .label(format!("{}.classify", domain.slug()))
        .json(json!({"type":"object","required":["data_type"]}))
        .complexity(Complexity::Simple)
}

pub fn extract_request(domain: Domain, dt: DataType, payload: &str) -> LlmRequest {
    // Structured extraction; escalates to Complex on high data density.
    LlmRequest::new(
        prompts::extractor_system(domain, dt),
        prompts::extract_task(dt, payload),
    )
    .label(format!("{}.extract", domain.slug()))
    .json(json!({"type":"object","required":["entities","relationships"]}))
    .complexity(Complexity::Standard)
    .density_aware()
}

pub fn correlate_request(domain: Domain, entities_json: &str) -> LlmRequest {
    LlmRequest::new(
        prompts::correlation_system(domain),
        prompts::correlate_task(entities_json),
    )
    .label(format!("{}.correlate", domain.slug()))
    .json(json!({"type":"object","required":["relationships"]}))
    .complexity(Complexity::Standard)
    .density_aware()
}

pub fn risk_request(domain: Domain, graph_summary: &str) -> LlmRequest {
    // Prioritization drives sensitive/irreversible decisions → Claude.
    LlmRequest::new(prompts::risk_system(domain), prompts::risk_task(graph_summary))
        .label(format!("{}.risk", domain.slug()))
        .json(json!({"type":"object","required":["assessments"]}))
        .complexity(Complexity::Complex)
}

pub fn investigate_request(domain: Domain, brief_input: &str) -> LlmRequest {
    // Deepest synthesis → Claude Opus.
    LlmRequest::new(
        prompts::investigator_system(domain),
        prompts::investigate_task(brief_input),
    )
    .label(format!("{}.investigate", domain.slug()))
    .json(json!({"type":"object","required":["summary"]}))
    .complexity(Complexity::Complex)
}

pub fn audit_request(domain: Domain, run_summary: &str) -> LlmRequest {
    LlmRequest::new(prompts::audit_system(domain), prompts::audit_task(run_summary))
        .label(format!("{}.audit", domain.slug()))
        .json(json!({"type":"object","required":["summary"]}))
        .complexity(Complexity::Simple)
}

pub fn ask_request(domain: Domain, question: &str, graph_context: &str) -> LlmRequest {
    // Interactive analyst reasoning → Claude (Opus on dense graphs).
    LlmRequest::new(
        prompts::analyst_system(domain),
        prompts::analyst_task(question, graph_context),
    )
    .label(format!("{}.ask", domain.slug()))
    .json(json!({"type":"object","required":["answer"]}))
    .complexity(Complexity::Complex)
}
