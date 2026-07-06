//! The ontology: entities, relationships and the in-memory knowledge graph.
//! Modeled after the object/property/link approach in DATA.md (§22, §"Ontology
//! inicial"). There is no local database — the graph lives in memory and is
//! serialized to JSON output files.

use indexmap::IndexMap;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The kinds of objects the graph can hold. Superset that covers every vertical
/// in DATA.md and DESIGN.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Case,
    Report,
    Person,
    Victim,
    Suspect,
    Account,
    Device,
    Ip,
    Url,
    Domain,
    Media,
    Evidence,
    Communication,
    Group,
    Payment,
    Wallet,
    Location,
    Organization,
    Malware,
    Vulnerability,
    Incident,
    Service,
    Repository,
    Unknown,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Case => "case",
            EntityKind::Report => "report",
            EntityKind::Person => "person",
            EntityKind::Victim => "victim",
            EntityKind::Suspect => "suspect",
            EntityKind::Account => "account",
            EntityKind::Device => "device",
            EntityKind::Ip => "ip",
            EntityKind::Url => "url",
            EntityKind::Domain => "domain",
            EntityKind::Media => "media",
            EntityKind::Evidence => "evidence",
            EntityKind::Communication => "communication",
            EntityKind::Group => "group",
            EntityKind::Payment => "payment",
            EntityKind::Wallet => "wallet",
            EntityKind::Location => "location",
            EntityKind::Organization => "organization",
            EntityKind::Malware => "malware",
            EntityKind::Vulnerability => "vulnerability",
            EntityKind::Incident => "incident",
            EntityKind::Service => "service",
            EntityKind::Repository => "repository",
            EntityKind::Unknown => "unknown",
        }
    }

    /// Parse a free-form kind label emitted by an LLM agent into a known kind.
    pub fn parse(s: &str) -> EntityKind {
        match s.trim().to_lowercase().as_str() {
            "case" => EntityKind::Case,
            "report" | "tip" | "complaint" => EntityKind::Report,
            "person" | "individual" | "people" => EntityKind::Person,
            "victim" => EntityKind::Victim,
            "suspect" | "offender" | "actor" | "threat_actor" => EntityKind::Suspect,
            "account" | "profile" | "user" | "username" | "handle" => EntityKind::Account,
            "device" | "host" | "endpoint" => EntityKind::Device,
            "ip" | "ip_address" | "ipv4" | "ipv6" => EntityKind::Ip,
            "url" | "link" | "uri" => EntityKind::Url,
            "domain" | "hostname" | "fqdn" => EntityKind::Domain,
            "media" | "file" | "image" | "video" | "audio" | "document" => EntityKind::Media,
            "evidence" | "exhibit" | "artifact" => EntityKind::Evidence,
            "communication" | "message" | "chat" | "email" => EntityKind::Communication,
            "group" | "channel" | "community" | "server" => EntityKind::Group,
            "payment" | "transaction" | "transfer" => EntityKind::Payment,
            "wallet" | "crypto_wallet" | "address" => EntityKind::Wallet,
            "location" | "place" | "geo" => EntityKind::Location,
            "organization" | "org" | "company" | "merchant" => EntityKind::Organization,
            "malware" | "sample" | "payload" => EntityKind::Malware,
            "vulnerability" | "cve" | "vuln" => EntityKind::Vulnerability,
            "incident" | "event" | "alert" => EntityKind::Incident,
            "service" | "cloud_service" | "saas" => EntityKind::Service,
            "repository" | "repo" | "github" => EntityKind::Repository,
            _ => EntityKind::Unknown,
        }
    }

    /// Sensitivity floor: some kinds always require restricted handling and
    /// human review before their contents are exposed (DATA.md §3, §16).
    pub fn is_sensitive(self) -> bool {
        matches!(
            self,
            EntityKind::Victim | EntityKind::Media | EntityKind::Evidence | EntityKind::Communication
        )
    }
}

/// Discrete risk band used across the UI and reports (DESIGN.md badges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskBand {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskBand {
    pub fn from_score(score: f32) -> RiskBand {
        match score {
            s if s >= 0.85 => RiskBand::Critical,
            s if s >= 0.6 => RiskBand::High,
            s if s >= 0.35 => RiskBand::Medium,
            _ => RiskBand::Low,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RiskBand::Low => "low",
            RiskBand::Medium => "medium",
            RiskBand::High => "high",
            RiskBand::Critical => "critical",
        }
    }
}

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub kind: EntityKind,
    /// Human-readable label (a redacted/safe display value).
    pub label: String,
    /// Normalized key used for deduplication (lowercased, canonical form).
    pub dedup_key: String,
    #[serde(default)]
    pub attributes: IndexMap<String, String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 0.0–1.0 continuous risk score; None until prioritization runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_band: Option<RiskBand>,
    /// Where this entity was first observed (source references).
    #[serde(default)]
    pub sources: Vec<String>,
    pub sensitive: bool,
    /// Identity resolution: alias entities merged into this canonical one.
    /// Each alias preserves its original label + provenance so the merge is
    /// reversible and auditable (Workstream E).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<Alias>,
}

/// An alias folded into a canonical entity by identity resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alias {
    pub label: String,
    pub sources: Vec<String>,
    /// Signals that justified merging this alias (explainable).
    pub signals: Vec<String>,
    pub confidence: f32,
}

impl Entity {
    pub fn new(kind: EntityKind, label: impl Into<String>) -> Entity {
        let label = label.into();
        let dedup_key = format!("{}::{}", kind.as_str(), normalize_key(&label));
        Entity {
            id: format!("ent-{}", uuid::Uuid::new_v4().simple()),
            kind,
            label,
            dedup_key,
            attributes: IndexMap::new(),
            tags: Vec::new(),
            risk_score: None,
            risk_band: None,
            sources: Vec::new(),
            sensitive: kind.is_sensitive(),
            aliases: Vec::new(),
        }
    }

    pub fn with_attr(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.attributes.insert(k.into(), v.into());
        self
    }

    pub fn with_source(mut self, s: impl Into<String>) -> Self {
        let s = s.into();
        if !self.sources.contains(&s) {
            self.sources.push(s);
        }
        self
    }

    /// Merge another entity (same dedup_key) into this one.
    pub fn merge(&mut self, other: &Entity) {
        for (k, v) in &other.attributes {
            self.attributes.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for t in &other.tags {
            if !self.tags.contains(t) {
                self.tags.push(t.clone());
            }
        }
        for s in &other.sources {
            if !self.sources.contains(s) {
                self.sources.push(s.clone());
            }
        }
        self.sensitive = self.sensitive || other.sensitive;
    }
}

/// A directed, typed link between two entities (DATA.md §22).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub rel_type: String,
    pub target_id: String,
    pub confidence: f32,
    #[serde(default)]
    pub source_reference: Option<String>,
    #[serde(default)]
    pub attributes: IndexMap<String, String>,
}

impl Relationship {
    pub fn new(
        source_id: impl Into<String>,
        rel_type: impl Into<String>,
        target_id: impl Into<String>,
        confidence: f32,
    ) -> Relationship {
        Relationship {
            id: format!("rel-{}", uuid::Uuid::new_v4().simple()),
            source_id: source_id.into(),
            rel_type: rel_type.into(),
            target_id: target_id.into(),
            confidence: confidence.clamp(0.0, 1.0),
            source_reference: None,
            attributes: IndexMap::new(),
        }
    }
}

/// In-memory knowledge graph with entity dedup and a petgraph view for
/// centrality / connected-component style analysis.
#[derive(Debug, Default)]
pub struct KnowledgeGraph {
    /// entity id -> Entity
    pub entities: IndexMap<String, Entity>,
    /// dedup_key -> entity id (for O(1) dedup on ingest)
    dedup_index: HashMap<String, String>,
    pub relationships: Vec<Relationship>,
    rel_index: HashMap<String, ()>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entity, deduplicating by canonical key. Returns the id of the
    /// canonical entity (either newly inserted or the pre-existing match).
    pub fn upsert_entity(&mut self, entity: Entity) -> String {
        if let Some(existing_id) = self.dedup_index.get(&entity.dedup_key).cloned() {
            if let Some(existing) = self.entities.get_mut(&existing_id) {
                existing.merge(&entity);
            }
            return existing_id;
        }
        let id = entity.id.clone();
        self.dedup_index.insert(entity.dedup_key.clone(), id.clone());
        self.entities.insert(id.clone(), entity);
        id
    }

    /// Add a relationship, skipping exact duplicates (same source/type/target).
    pub fn add_relationship(&mut self, rel: Relationship) {
        let key = format!("{}|{}|{}", rel.source_id, rel.rel_type, rel.target_id);
        if self.rel_index.contains_key(&key) {
            return;
        }
        self.rel_index.insert(key, ());
        self.relationships.push(rel);
    }

    /// Identity resolution: fold `alias_id` into `canonical_id`. Reconnects the
    /// alias's relationships to the canonical entity, records the alias (with its
    /// provenance + the signals that justified the merge) so it is reversible,
    /// and removes the alias node. No-op if either id is missing or they're equal.
    pub fn merge_entities(&mut self, canonical_id: &str, alias_id: &str, signals: Vec<String>, confidence: f32) -> bool {
        if canonical_id == alias_id || !self.entities.contains_key(canonical_id) || !self.entities.contains_key(alias_id) {
            return false;
        }
        let alias = self.entities.get(alias_id).unwrap().clone();
        // Record the alias on the canonical entity.
        if let Some(c) = self.entities.get_mut(canonical_id) {
            for (k, v) in &alias.attributes {
                c.attributes.entry(k.clone()).or_insert_with(|| v.clone());
            }
            for s in &alias.sources {
                if !c.sources.contains(s) { c.sources.push(s.clone()); }
            }
            // carry the alias's own nested aliases up too
            let mut all_sources = alias.sources.clone();
            for a in &alias.aliases { all_sources.extend(a.sources.clone()); c.aliases.push(a.clone()); }
            c.aliases.push(Alias { label: alias.label.clone(), sources: all_sources, signals, confidence });
            if !c.tags.contains(&"resolved-identity".to_string()) { c.tags.push("resolved-identity".into()); }
        }
        // Rewire relationships from alias → canonical, dropping self-loops/dupes.
        self.rel_index.clear();
        let mut kept: Vec<Relationship> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let taken = std::mem::take(&mut self.relationships);
        for mut r in taken {
            if r.source_id == alias_id { r.source_id = canonical_id.to_string(); }
            if r.target_id == alias_id { r.target_id = canonical_id.to_string(); }
            if r.source_id == r.target_id { continue; }
            let key = format!("{}|{}|{}", r.source_id, r.rel_type, r.target_id);
            if seen.insert(key.clone()) { self.rel_index.insert(key, ()); kept.push(r); }
        }
        self.relationships = kept;
        // Remove the alias node from the entity map + dedup index.
        self.entities.shift_remove(alias_id);
        self.dedup_index.retain(|_, v| v != alias_id);
        true
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    /// Build a petgraph directed graph for structural analysis.
    #[allow(dead_code)]
    pub fn to_petgraph(&self) -> (DiGraph<String, String>, HashMap<String, NodeIndex>) {
        let mut g = DiGraph::<String, String>::new();
        let mut idx: HashMap<String, NodeIndex> = HashMap::new();
        for id in self.entities.keys() {
            let n = g.add_node(id.clone());
            idx.insert(id.clone(), n);
        }
        for rel in &self.relationships {
            if let (Some(&a), Some(&b)) = (idx.get(&rel.source_id), idx.get(&rel.target_id)) {
                g.add_edge(a, b, rel.rel_type.clone());
            }
        }
        (g, idx)
    }

    /// Degree centrality (in+out) per entity id — a cheap importance signal
    /// used both as a risk feature and to surface hub entities.
    pub fn degree_centrality(&self) -> HashMap<String, usize> {
        let mut deg: HashMap<String, usize> = self.entities.keys().map(|k| (k.clone(), 0)).collect();
        for rel in &self.relationships {
            *deg.entry(rel.source_id.clone()).or_insert(0) += 1;
            *deg.entry(rel.target_id.clone()).or_insert(0) += 1;
        }
        deg
    }
}

/// Canonicalize a label into a dedup key: lowercase, trim, collapse whitespace.
pub fn normalize_key(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
