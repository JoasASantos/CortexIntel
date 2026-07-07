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
    /// KYC / identity verification & fraud analysis.
    Kyc,
    /// Healthcare operations & clinical-safety intelligence.
    Health,
    /// Retail / e-commerce decisioning.
    Commerce,
    /// Supply chain & logistics operations.
    Logistics,
    /// Military / defense intelligence.
    Military,
    /// Government / public-sector intelligence.
    Government,
    /// Banking & finance.
    Finance,
    /// Insurance underwriting & claims.
    Insurance,
    /// Telecom operations & abuse.
    Telecom,
    /// Energy & critical infrastructure.
    Energy,
    /// Legal / e-discovery / compliance.
    Legal,
    /// Manufacturing & industrial ops.
    Manufacturing,
    /// Real estate & property intelligence.
    RealEstate,
    /// Education & academic institutions.
    Education,
    /// Nonprofit / hotline / humanitarian.
    Nonprofit,
    /// Domain-neutral: generic entity/relationship intelligence.
    Generic,
}

impl Domain {
    pub fn slug(self) -> &'static str {
        match self {
            Domain::ChildProtection => "child-protection",
            Domain::Cybersecurity => "cybersecurity",
            Domain::Fraud => "fraud",
            Domain::Kyc => "kyc",
            Domain::Health => "health",
            Domain::Commerce => "commerce",
            Domain::Logistics => "logistics",
            Domain::Military => "military",
            Domain::Government => "government",
            Domain::Finance => "finance",
            Domain::Insurance => "insurance",
            Domain::Telecom => "telecom",
            Domain::Energy => "energy",
            Domain::Legal => "legal",
            Domain::Manufacturing => "manufacturing",
            Domain::RealEstate => "real-estate",
            Domain::Education => "education",
            Domain::Nonprofit => "nonprofit",
            Domain::Generic => "generic",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Domain::ChildProtection => "Child Protection & Victim Identification",
            Domain::Cybersecurity => "Cybersecurity / Threat Intelligence",
            Domain::Fraud => "Fraud, AML & Financial Crime",
            Domain::Kyc => "KYC / Identity Verification & Fraud Analysis",
            Domain::Health => "Healthcare & Clinical Safety",
            Domain::Commerce => "Commerce & Retail Decisioning",
            Domain::Logistics => "Logistics & Supply Chain",
            Domain::Military => "Military & Defense Intelligence",
            Domain::Government => "Government & Public Sector",
            Domain::Finance => "Banking & Finance",
            Domain::Insurance => "Insurance (Underwriting & Claims)",
            Domain::Telecom => "Telecommunications",
            Domain::Energy => "Energy & Critical Infrastructure",
            Domain::Legal => "Legal, e-Discovery & Compliance",
            Domain::Manufacturing => "Manufacturing & Industrial",
            Domain::RealEstate => "Real Estate & Property",
            Domain::Education => "Education & Academia",
            Domain::Nonprofit => "Nonprofit & Humanitarian",
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
            Domain::Kyc => "Verify identities and analyze fraud. Correlate a person's connected records/documents to assess whether the identity behind a document is real, with country-aware checks. Lawful basis required; respect LGPD/GDPR — validation is decision-support, never a definitive identity ruling.",
            Domain::Health => "Improve clinical and operational safety. Correlate events and signals to flag risk while protecting patient privacy at all times.",
            Domain::Commerce => "Improve commercial decisions. Correlate customers, orders, channels and signals to surface risk and opportunity.",
            Domain::Logistics => "Optimize and de-risk operations. Correlate shipments, routes, assets and disruptions to recommend resilient actions.",
            Domain::Military => "Support defense intelligence and force protection. Correlate actors, units, infrastructure, movements and signals into decision-ready, human-reviewed assessments. Never automate targeting or lethal decisions.",
            Domain::Government => "Support public-sector analysis and safety. Correlate records, entities and events for lawful, accountable decision-making.",
            Domain::Finance => "Detect financial crime and manage exposure. Correlate accounts, counterparties and transactions to quantify risk.",
            Domain::Insurance => "Assess risk and detect claims fraud. Correlate policies, claims, parties and evidence.",
            Domain::Telecom => "Detect abuse and map infrastructure. Correlate subscribers, numbers, devices and network events.",
            Domain::Energy => "Protect critical infrastructure. Correlate assets, access, incidents and threats to prioritize resilience.",
            Domain::Legal => "Support e-discovery and compliance. Correlate custodians, communications and documents into defensible findings.",
            Domain::Manufacturing => "Improve industrial operations & safety. Correlate assets, suppliers, sensors and incidents.",
            Domain::RealEstate => "Property & ownership intelligence. Correlate entities, holdings, transactions and locations.",
            Domain::Education => "Institutional & student intelligence. Correlate people, enrollments and activity while protecting privacy.",
            Domain::Nonprofit => "Support humanitarian and hotline work. Correlate reports and cases to protect people, with strict data minimization.",
            Domain::Generic => "Turn heterogeneous data into a correlated, prioritized, auditable picture that supports human decision-making.",
        }
    }

    pub fn all() -> &'static [Domain] {
        &[
            Domain::ChildProtection,
            Domain::Cybersecurity,
            Domain::Fraud,
            Domain::Kyc,
            Domain::Health,
            Domain::Commerce,
            Domain::Logistics,
            Domain::Military,
            Domain::Government,
            Domain::Finance,
            Domain::Insurance,
            Domain::Telecom,
            Domain::Energy,
            Domain::Legal,
            Domain::Manufacturing,
            Domain::RealEstate,
            Domain::Education,
            Domain::Nonprofit,
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

    /// UI grouping category for the data-type selector.
    pub fn category(self) -> &'static str {
        match self {
            DataType::Case | DataType::Report | DataType::Event | DataType::Log => "Cases & Events",
            DataType::Person | DataType::Account | DataType::Customer | DataType::Student | DataType::Employee => "People & Accounts",
            DataType::Device | DataType::Network | DataType::Url => "Digital & Network",
            DataType::Media | DataType::Communication => "Content & Media",
            DataType::Financial | DataType::Order | DataType::Product => "Commercial & Financial",
            DataType::Shipment | DataType::Asset | DataType::Sensor | DataType::Location => "Operations & Assets",
            DataType::Generic => "Other",
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
    /// UI language for generated intelligence text ("en" | "pt" | "es").
    pub lang: String,
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
            lang: "en".into(),
        }
    }
}

fn whoami_fallback() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "operator".into())
}
