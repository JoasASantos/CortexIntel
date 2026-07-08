//! Agent library: ready-made agents defined as **markdown files** (frontmatter +
//! prompt body), not hard-coded. This scales to thousands of agents for every
//! case type — you just drop `.md` files in the agents directory. Agents are
//! generic (`domains: ["*"]`) or scoped to project types, and are listed,
//! searched and dispatched from the UI; running one sends its body through the
//! LLM against the current graph + project specs, and results reflect back.
//!
//! Location: `$CORTEX_AGENTS_DIR` or `<data-dir>/agents/` (recursed). A starter
//! set is seeded on first use so the library is never empty.

use serde::Serialize;
use std::path::PathBuf;

/// An input field an agent can request before running (opens a form in the UI).
/// Values are substituted into the body as `{{name}}`.
#[derive(Debug, Clone, Serialize)]
pub struct AgentInput {
    pub name: String,
    pub label: String,
    /// "text" | "number" | "select".
    pub kind: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Verticals this applies to; `["*"]` = generic/universal.
    pub domains: Vec<String>,
    /// Finer grouping within a niche (e.g. finance → "AML", "KYC", "transactions").
    pub category: String,
    pub tags: Vec<String>,
    /// Entity kinds / tags whose presence in the data makes this agent relevant
    /// (drives recommendation). Empty = always eligible for its domain.
    #[serde(default)]
    pub triggers: Vec<String>,
    /// Optional input fields — when present, running opens a form first.
    #[serde(default)]
    pub inputs: Vec<AgentInput>,
    /// Optional graph view to switch to after running ("network"|"map"|"timeline").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub view: String,
    /// Recommendation score for the current data (0 unless recommend() ran).
    #[serde(default)]
    pub score: f32,
    /// Run automatically after ingestion?
    pub auto: bool,
    /// What running it produces: "answer" | "graph" | "focus".
    pub reflects: String,
    /// The prompt/task body (markdown). Skipped in list responses.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub body: String,
}

fn agents_dir() -> PathBuf {
    if let Ok(p) = std::env::var("CORTEX_AGENTS_DIR") {
        return PathBuf::from(p);
    }
    crate::store::base_dir().join("agents")
}

/// Parse one markdown agent (frontmatter between `---` fences + body).
fn parse_agent(id: &str, text: &str) -> Option<Agent> {
    let text = text.trim_start();
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let (front, body) = rest.split_at(end);
    let body = body.trim_start_matches("\n---").trim().to_string();

    let mut name = id.to_string();
    let mut description = String::new();
    let mut domains: Vec<String> = vec!["*".into()];
    let mut category = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut triggers: Vec<String> = Vec::new();
    let mut inputs: Vec<AgentInput> = Vec::new();
    let mut view = String::new();
    let mut auto = false;
    let mut reflects = "answer".to_string();

    let list = |v: &str| -> Vec<String> {
        v.trim().trim_start_matches('[').trim_end_matches(']')
            .split(',').map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty()).collect()
    };
    for line in front.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "name" => name = v.trim_matches('"').to_string(),
            "description" => description = v.trim_matches('"').to_string(),
            "domains" | "domain" => { let l = list(v); if !l.is_empty() { domains = l; } }
            "category" => category = v.trim_matches('"').to_string(),
            "tags" => tags = list(v),
            "triggers" => triggers = list(v).into_iter().map(|s| s.to_lowercase()).collect(),
            "view" => view = v.trim_matches('"').to_string(),
            // inputs: [name:type:Label, amount:number:Minimum, status:select:Status:open|closed]
            "inputs" => {
                for spec in list(v) {
                    let parts: Vec<&str> = spec.split(':').collect();
                    if parts.is_empty() || parts[0].is_empty() { continue; }
                    let options = parts.get(3).map(|o| o.split('|').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
                    inputs.push(AgentInput {
                        name: parts[0].trim().to_string(),
                        kind: parts.get(1).map(|s| s.trim().to_string()).unwrap_or_else(|| "text".into()),
                        label: parts.get(2).map(|s| s.trim().to_string()).unwrap_or_else(|| parts[0].trim().to_string()),
                        options,
                    });
                }
            }
            "auto" => auto = v == "true",
            "reflects" => reflects = v.trim_matches('"').to_string(),
            _ => {}
        }
    }
    if body.is_empty() {
        return None;
    }
    Some(Agent { id: id.to_string(), name, description, domains, category, tags, triggers, inputs, view, score: 0.0, auto, reflects, body })
}

/// Recursively collect `.md` files under `dir`.
fn walk_md(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_md(&p, out);
        } else if p.extension().map(|x| x == "md").unwrap_or(false) {
            out.push(p);
        }
    }
}

/// Load every agent in the library (seeding the starter set if empty).
pub fn load_all() -> Vec<Agent> {
    let dir = agents_dir();
    if !dir.exists() || std::fs::read_dir(&dir).map(|mut r| r.next().is_none()).unwrap_or(true) {
        seed_starters(&dir);
    }
    let mut files = Vec::new();
    walk_md(&dir, &mut files);
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let id = f.file_stem().and_then(|s| s.to_str()).unwrap_or("agent").to_string();
        if let Ok(text) = std::fs::read_to_string(&f) {
            if let Some(a) = parse_agent(&id, &text) {
                out.push(a);
            }
        }
    }
    out
}

/// List agents for a vertical, optionally filtered by a search query. Bodies are
/// stripped (metadata only) so the list stays light even with thousands present.
pub fn library(domain: &str, query: &str) -> Vec<Agent> {
    let q = query.trim().to_lowercase();
    load_all()
        .into_iter()
        .filter(|a| a.domains.iter().any(|d| d == "*" || d.eq_ignore_ascii_case(domain)))
        .filter(|a| {
            q.is_empty()
                || a.name.to_lowercase().contains(&q)
                || a.description.to_lowercase().contains(&q)
                || a.tags.iter().any(|t| t.to_lowercase().contains(&q))
                || a.id.to_lowercase().contains(&q)
        })
        .map(|mut a| { a.body = String::new(); a })
        .collect()
}

/// Recommend agents for the CURRENT data: understands what's present (entity
/// kinds/tags) and ranks agents whose triggers match, biased to the domain.
/// `present` is the set of entity kinds + salient tags found in the graph.
pub fn recommend(domain: &str, present: &[String], limit: usize) -> Vec<Agent> {
    let present: std::collections::HashSet<String> = present.iter().map(|s| s.to_lowercase()).collect();
    let mut scored: Vec<Agent> = load_all()
        .into_iter()
        .filter(|a| a.domains.iter().any(|d| d == "*" || d.eq_ignore_ascii_case(domain)))
        .map(|mut a| {
            let mut s = 0.0f32;
            // Domain fit: exact niche match beats generic.
            if a.domains.iter().any(|d| d.eq_ignore_ascii_case(domain)) { s += 2.0; }
            else if a.domains.iter().any(|d| d == "*") { s += 0.5; }
            // Data fit: how many of the agent's triggers are present in the data.
            let hits = a.triggers.iter().filter(|t| present.contains(*t)).count();
            s += hits as f32 * 2.0;
            // A trigger-less generic agent still has baseline utility.
            if a.triggers.is_empty() { s += 0.5; }
            if a.auto { s += 0.5; }
            a.score = s;
            a.body = String::new();
            a
        })
        .filter(|a| a.score > 0.0)
        .collect();
    scored.sort_by(|x, y| y.score.partial_cmp(&x.score).unwrap());
    scored.truncate(limit);
    scored
}

/// Fetch a single agent by id, including its prompt body (to dispatch).
pub fn get(id: &str) -> Option<Agent> {
    load_all().into_iter().find(|a| a.id == id)
}

/// Sanitize an id into a safe filename stem (no path traversal).
fn safe_id(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>().trim_matches('-').to_lowercase()
}

/// Create or overwrite an agent markdown file. `content` is the full markdown
/// (frontmatter + body). Returns the parsed agent, or an error if it won't parse.
pub fn save(id: &str, content: &str) -> Result<Agent, String> {
    let id = safe_id(id);
    if id.is_empty() {
        return Err("invalid agent id".into());
    }
    let parsed = parse_agent(&id, content).ok_or("markdown must have frontmatter (--- … ---) and a body")?;
    let dir = agents_dir();
    // ensure the library exists / is seeded before writing a custom agent
    if !dir.exists() { let _ = std::fs::create_dir_all(&dir); }
    std::fs::write(dir.join(format!("{id}.md")), content).map_err(|e| e.to_string())?;
    Ok(parsed)
}

/// Delete a custom agent file by id. No-op for the bundled starters if they were
/// re-seeded (they can be overridden by saving over them).
pub fn delete(id: &str) -> Result<(), String> {
    let id = safe_id(id);
    let p = agents_dir().join(format!("{id}.md"));
    if p.exists() { std::fs::remove_file(&p).map_err(|e| e.to_string())?; }
    Ok(())
}

/// Write the starter agents to disk on first use.
fn seed_starters(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    for (name, content) in STARTERS {
        let p = dir.join(name);
        if !p.exists() {
            let _ = std::fs::write(&p, content);
        }
    }
}

/// Bundled starter agents — a small, real set spanning generic + per-vertical.
/// The library is meant to grow to thousands by dropping more `.md` files here.
const STARTERS: &[(&str, &str)] = &[
    ("executive-brief.md", include_str!("../agents/executive-brief.md")),
    ("hidden-links.md", include_str!("../agents/hidden-links.md")),
    ("key-players.md", include_str!("../agents/key-players.md")),
    ("intel-gaps.md", include_str!("../agents/intel-gaps.md")),
    ("anomalies.md", include_str!("../agents/anomalies.md")),
    ("identity-resolution.md", include_str!("../agents/identity-resolution.md")),
    ("timeline-reconstruction.md", include_str!("../agents/timeline-reconstruction.md")),
    ("entity-deep-dive.md", include_str!("../agents/entity-deep-dive.md")),
    ("contradiction-check.md", include_str!("../agents/contradiction-check.md")),
    ("data-quality-audit.md", include_str!("../agents/data-quality-audit.md")),
    ("narrative-report.md", include_str!("../agents/narrative-report.md")),
    ("victim-identification.md", include_str!("../agents/victim-identification.md")),
    ("takedown-priority.md", include_str!("../agents/takedown-priority.md")),
    ("grooming-escalation.md", include_str!("../agents/grooming-escalation.md")),
    ("csam-hash-triage.md", include_str!("../agents/csam-hash-triage.md")),
    ("ttp-map.md", include_str!("../agents/ttp-map.md")),
    ("ioc-enrichment.md", include_str!("../agents/ioc-enrichment.md")),
    ("lateral-movement.md", include_str!("../agents/lateral-movement.md")),
    ("c2-detection.md", include_str!("../agents/c2-detection.md")),
    ("money-flow.md", include_str!("../agents/money-flow.md")),
    ("aml-layering.md", include_str!("../agents/aml-layering.md")),
    ("kyc-plausibility.md", include_str!("../agents/kyc-plausibility.md")),
    ("mule-network.md", include_str!("../agents/mule-network.md")),
    ("sanctions-screening.md", include_str!("../agents/sanctions-screening.md")),
    ("chargeback-ring.md", include_str!("../agents/chargeback-ring.md")),
    ("logistics-bottleneck.md", include_str!("../agents/logistics-bottleneck.md")),
    ("route-disruption.md", include_str!("../agents/route-disruption.md")),
    ("supplier-dependency.md", include_str!("../agents/supplier-dependency.md")),
    ("adverse-event-cluster.md", include_str!("../agents/adverse-event-cluster.md")),
    ("outbreak-linkage.md", include_str!("../agents/outbreak-linkage.md")),
    ("churn-risk.md", include_str!("../agents/churn-risk.md")),
    ("segment-discovery.md", include_str!("../agents/segment-discovery.md")),
    ("simbox-detection.md", include_str!("../agents/simbox-detection.md")),
    ("critical-asset-mapping.md", include_str!("../agents/critical-asset-mapping.md")),
    ("claims-fraud-ring.md", include_str!("../agents/claims-fraud-ring.md")),
    ("procurement-collusion.md", include_str!("../agents/procurement-collusion.md")),
    ("comms-threads.md", include_str!("../agents/comms-threads.md")),
    ("coa-analysis.md", include_str!("../agents/coa-analysis.md")),
    ("follow-the-money.md", include_str!("../agents/follow-the-money.md")),
    ("conflict-of-interest.md", include_str!("../agents/conflict-of-interest.md")),
    ("source-corroboration.md", include_str!("../agents/source-corroboration.md")),
    ("background-dossier.md", include_str!("../agents/background-dossier.md")),
    ("document-linkage.md", include_str!("../agents/document-linkage.md")),
    ("public-interest-timeline.md", include_str!("../agents/public-interest-timeline.md")),
    ("influence-network.md", include_str!("../agents/influence-network.md")),
];
