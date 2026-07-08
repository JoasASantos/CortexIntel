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
    // ---- Generalist agents (domains: ["*"]) — robust, work on any case ----
    ("executive-brief.md", r#"---
name: Executive Brief
description: One-paragraph decision-ready assessment of the whole case, plus the single next action.
domains: ["*"]
category: Overview
tags: [summary, decision]
auto: true
reflects: answer
---
Act as the lead analyst. In one short paragraph, give the decision-maker the
picture: what is going on, how confident you are, and why it matters. Then state
the ONE next action that most moves the decision. No jargon.
"#),
    ("hidden-links.md", r#"---
name: Hidden Links Finder
description: Propose relationships that are likely but not directly present in the data.
domains: ["*"]
category: Network
tags: [network, inference]
reflects: graph
view: network
---
Examine the graph for entities that are probably connected but have no direct
edge (shared neighbours, matching attributes, timing). Propose those links as
relationships with a confidence and a one-line justification. Never assert a link
without evidence in the graph.
"#),
    ("key-players.md", r#"---
name: Key Players & Clusters
description: Identify the brokers/ringleaders and the tight clusters holding the network together.
domains: ["*"]
category: Network
tags: [network, centrality]
reflects: focus
view: network
---
Identify the key players: who bridges otherwise separate parts of the network
(brokers), who is most central, and which tight clusters exist. Explain who holds
the network together and what removing each key node would do.
"#),
    ("intel-gaps.md", r#"---
name: Intelligence Gaps
description: What's missing to decide, and the single collection step that most cuts uncertainty.
domains: ["*"]
category: Decision
tags: [collection, next-best-action]
reflects: answer
---
List the critical unknowns that block a confident decision. Then recommend the
single collection or verification step that would most reduce uncertainty, and
say why.
"#),
    ("anomalies.md", r#"---
name: Anomaly Reviewer
description: Surface anything out of pattern and explain why it matters.
domains: ["*"]
category: Quality
tags: [anomaly, review]
reflects: focus
---
Point out entities or relationships that are out of pattern relative to the rest
of the case (unusual connectivity, timing, attributes). For each, explain whether
it looks like a genuine signal, a data-quality artifact, or a benign outlier.
"#),
    ("identity-resolution.md", r#"---
name: Identity Resolution Review
description: Find aliases/accounts that are probably the same person and explain the signals.
domains: ["*"]
category: Identity
tags: [identity, dedup]
triggers: [person, account, device, victim, suspect]
reflects: graph
---
Review the case for entities that are probably the same real-world identity across
aliases, accounts, devices or contacts. For each candidate merge, list the signals
that justify it and a confidence. Flag ambiguous cases for human review rather than
merging blindly.
"#),
    // ---- Child protection ----
    ("victim-identification.md", r#"---
name: Victim Identification Support
description: Correlate victim indicators and flag matches against known references. Human-review only.
domains: [child-protection]
category: Victim ID
tags: [victim, identification]
triggers: [victim, media, person]
auto: true
reflects: answer
---
Support victim identification. Correlate the victim indicators in the case,
highlight any match against integrated reference sources, and note what would
confirm an identification. This is decision-support requiring human confirmation;
never expose or reproduce sensitive media — reference it by hash/id only.
"#),
    ("takedown-priority.md", r#"---
name: Takedown Priority
description: Rank distribution/infrastructure nodes by takedown priority.
domains: [child-protection, cybersecurity]
category: Disruption
tags: [takedown, infrastructure]
triggers: [domain, url, ip, account]
reflects: focus
view: network
---
Rank the distribution and infrastructure nodes (domains, URLs, hosting, accounts)
by takedown priority: reach, centrality, and how much disruption removing each
would cause. Give the top targets and the reasoning.
"#),
    // ---- Finance / fraud / AML / KYC (niche with categories) ----
    ("money-flow.md", r#"---
name: Money-Flow Tracer
description: Trace funds across accounts/wallets; surface mule structures and exposure.
domains: [fraud, finance, kyc]
category: Financial Crime
tags: [financial-crime, network]
triggers: [wallet, payment, account]
inputs: [min_amount:number:Minimum amount to trace]
auto: true
reflects: graph
view: network
---
Trace money movement in the graph{{min_amount}}: chains of payments/transfers,
accounts or wallets that likely share a controller (mule structures), and total
exposure. Propose the missing links between counterparties, with confidence.
"#),
    ("aml-layering.md", r#"---
name: AML Layering Detector
description: Detect layering/structuring patterns across transactions in a time window.
domains: [fraud, finance]
category: AML
tags: [aml, laundering, temporal]
triggers: [payment, wallet, account]
inputs: [window_days:number:Look-back window (days), threshold:number:Structuring threshold]
reflects: graph
view: timeline
---
Look for layering and structuring: many small transfers splitting a larger sum,
rapid movement across accounts, or amounts just under a reporting threshold
({{threshold}}) within the last {{window_days}} days. Flag the chains and the
accounts that anchor them.
"#),
    ("kyc-plausibility.md", r#"---
name: KYC Plausibility Check
description: Assess whether a person's connected records form a plausible, consistent identity.
domains: [kyc, finance]
category: KYC
tags: [kyc, identity]
triggers: [person, account, location]
reflects: answer
---
Assess identity plausibility (country-aware, respecting LGPD/GDPR): do the
person's connected accounts, devices, locations and documents form a consistent
picture, or are there contradictions that suggest synthetic/stolen identity?
Decision-support only.
"#),
    // ---- Cybersecurity ----
    ("ttp-map.md", r#"---
name: Infrastructure & TTP Map
description: Map actors, infrastructure and TTPs; suggest hunting leads and containment.
domains: [cybersecurity]
category: Threat Intel
tags: [threat-intel, ttp]
triggers: [ip, domain, url, malware, incident]
auto: true
reflects: graph
view: network
---
Map the actors, infrastructure (IPs, domains, hosts) and techniques present in the
case. Cluster shared infrastructure, suggest hunting leads to expand coverage, and
recommend containment steps. Propose likely infrastructure links with confidence.
"#),
    // ---- Logistics ----
    ("logistics-bottleneck.md", r#"---
name: Bottleneck & Single-Point-of-Failure
description: Find chokepoints and dependencies whose failure would disrupt the operation.
domains: [logistics]
category: Operations
tags: [resilience, network]
triggers: [location, device, incident, organization]
reflects: focus
view: network
---
Model the operation as a network and find bottlenecks and single points of failure:
nodes that many routes depend on, and whose disruption would cascade. Recommend
resilient alternatives.
"#),
];
