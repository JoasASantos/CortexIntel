//! Configuration primitives: business verticals (domains), data-type classes,
//! provider routing and pipeline options. CortexIntel is domain-agnostic — the
//! child-protection ontology in DATA.md is one preset among several verticals.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Business vertical the platform is operating for. Drives which agents and
/// prompt presets are loaded, and how risk features are weighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Domain {
    /// Child-protection / hotline / victim-identification (the DATA.md preset).
    ChildProtection,
    /// SOC / threat-intel / DFIR / red & blue team.
    Cybersecurity,
    /// Fraud, AML, financial crime investigation.
    Fraud,
    /// Healthcare operations & clinical-safety intelligence.
    Health,
    /// Retail / e-commerce decisioning.
    Commerce,
    /// Supply chain & logistics operations.
    Logistics,
    /// Domain-neutral: generic entity/relationship intelligence.
    Generic,
}

impl Domain {
    pub fn slug(self) -> &'static str {
        match self {
            Domain::ChildProtection => "child-protection",
            Domain::Cybersecurity => "cybersecurity",
            Domain::Fraud => "fraud",
            Domain::Health => "health",
            Domain::Commerce => "commerce",
            Domain::Logistics => "logistics",
            Domain::Generic => "generic",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Domain::ChildProtection => "Child Protection & Victim Identification",
            Domain::Cybersecurity => "Cybersecurity / Threat Intelligence",
            Domain::Fraud => "Fraud, AML & Financial Crime",
            Domain::Health => "Healthcare & Clinical Safety",
            Domain::Commerce => "Commerce & Retail Decisioning",
            Domain::Logistics => "Logistics & Supply Chain",
            Domain::Generic => "Generic Intelligence",
        }
    }

    /// A short mission statement injected into every agent's system prompt so the
    /// LLM stays anchored to the vertical's goals and guardrails.
    pub fn mission(self) -> &'static str {
        match self {
            Domain::ChildProtection => "Protect children and identify victims. Prioritize imminent-risk cases, preserve chain of custody, and never expose sensitive material beyond operational need. The AI supports investigators; it never decides guilt.",
            Domain::Cybersecurity => "Detect, correlate and prioritize threats. Map infrastructure, actors and TTPs; surface actionable, evidence-backed leads for SOC/DFIR analysts.",
            Domain::Fraud => "Detect fraud rings and illicit money flows. Correlate accounts, devices and transactions; quantify exposure and recommend defensible next steps.",
            Domain::Health => "Improve clinical and operational safety. Correlate events and signals to flag risk while protecting patient privacy at all times.",
            Domain::Commerce => "Improve commercial decisions. Correlate customers, orders, channels and signals to surface risk and opportunity.",
            Domain::Logistics => "Optimize and de-risk operations. Correlate shipments, routes, assets and disruptions to recommend resilient actions.",
            Domain::Generic => "Turn heterogeneous data into a correlated, prioritized, auditable picture that supports human decision-making.",
        }
    }

    pub fn all() -> &'static [Domain] {
        &[
            Domain::ChildProtection,
            Domain::Cybersecurity,
            Domain::Fraud,
            Domain::Health,
            Domain::Commerce,
            Domain::Logistics,
            Domain::Generic,
        ]
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title())
    }
}

/// High-level class of an incoming record. Determines which classification and
/// entity-extraction agent handles a batch. Kept intentionally broad so any
/// vertical can map its raw feeds onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DataType {
    /// A case / investigation record.
    Case,
    /// A report / tip / complaint / hotline submission.
    Report,
    /// Digital media / evidence reference (hashes, fingerprints).
    Media,
    /// Online account / profile.
    Account,
    /// A natural person.
    Person,
    /// A device.
    Device,
    /// Network telemetry (IP, ASN, flows).
    Network,
    /// URL / domain / infrastructure.
    Url,
    /// Communications / messages.
    Communication,
    /// Financial transactions / wallets.
    Financial,
    /// Geospatial location.
    Location,
    /// Customer / CRM record.
    Customer,
    /// Student / learner / enrollment record.
    Student,
    /// Employee / HR record.
    Employee,
    /// Product / SKU / catalog item.
    Product,
    /// Order / sale / booking.
    Order,
    /// Shipment / logistics movement.
    Shipment,
    /// Asset / inventory item.
    Asset,
    /// Sensor / IoT telemetry.
    Sensor,
    /// Log / audit / event stream.
    Log,
    /// Generic event.
    Event,
    /// Anything else — the generic extractor infers structure.
    Generic,
}

impl DataType {
    pub fn slug(self) -> &'static str {
        match self {
            DataType::Case => "case",
            DataType::Report => "report",
            DataType::Media => "media",
            DataType::Account => "account",
            DataType::Person => "person",
            DataType::Device => "device",
            DataType::Network => "network",
            DataType::Url => "url",
            DataType::Communication => "communication",
            DataType::Financial => "financial",
            DataType::Location => "location",
            DataType::Customer => "customer",
            DataType::Student => "student",
            DataType::Employee => "employee",
            DataType::Product => "product",
            DataType::Order => "order",
            DataType::Shipment => "shipment",
            DataType::Asset => "asset",
            DataType::Sensor => "sensor",
            DataType::Log => "log",
            DataType::Event => "event",
            DataType::Generic => "generic",
        }
    }

    pub fn all() -> &'static [DataType] {
        &[
            DataType::Case,
            DataType::Report,
            DataType::Media,
            DataType::Account,
            DataType::Person,
            DataType::Device,
            DataType::Network,
            DataType::Url,
            DataType::Communication,
            DataType::Financial,
            DataType::Location,
            DataType::Customer,
            DataType::Student,
            DataType::Employee,
            DataType::Product,
            DataType::Order,
            DataType::Shipment,
            DataType::Asset,
            DataType::Sensor,
            DataType::Log,
            DataType::Event,
            DataType::Generic,
        ]
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.slug())
    }
}

/// Which LLM backend the router should prefer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderChoice {
    /// Claude Code CLI (subscription, `--dangerously-skip-permissions`).
    Claude,
    /// ChatGPT Codex CLI (`codex exec`).
    Codex,
    /// Try Claude first, fall back to Codex on failure.
    Auto,
    /// Deterministic offline stub — no external calls, no cost.
    Mock,
}

impl fmt::Display for ProviderChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProviderChoice::Claude => "claude",
            ProviderChoice::Codex => "codex",
            ProviderChoice::Auto => "auto",
            ProviderChoice::Mock => "mock",
        };
        write!(f, "{s}")
    }
}

/// Runtime options threaded through the pipeline.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub domain: Domain,
    pub data_type: Option<DataType>,
    pub provider: ProviderChoice,
    pub claude_model: Option<String>,
    pub codex_model: Option<String>,
    pub output_dir: std::path::PathBuf,
    pub operator: String,
    pub legal_basis: String,
    pub retention_days: i64,
    /// Cap on records ingested (protects the graph on huge feeds). None = all.
    pub max_records: Option<usize>,
    /// Do not call any real LLM; used for CI / smoke tests.
    pub offline: bool,
    pub verbose: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig {
            domain: Domain::Generic,
            data_type: None,
            provider: ProviderChoice::Auto,
            claude_model: None,
            codex_model: None,
            output_dir: std::path::PathBuf::from("./cortex-out"),
            operator: whoami_fallback(),
            legal_basis: "internal_authorization".into(),
            retention_days: 365,
            max_records: None,
            offline: false,
            verbose: false,
        }
    }
}

fn whoami_fallback() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "operator".into())
}
