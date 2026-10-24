//! Orchestrator: dynamic tool registry + planner → worker → critic loop.
//!
//! An analyst question is not sent raw to a big model. Instead:
//!   1. **Planner** (cheap/fast model, or a deterministic fallback) picks which
//!      tools to run from the registry — retrieval, grep, graph queries,
//!      installed transforms — and how to answer.
//!   2. **Worker** executes the plan locally (no LLM), compressing every
//!      observation before it is kept.
//!   3. **Answer** (the tier the router picks for Complex reasoning) gets only
//!      the GraphRAG context + compressed observations — never the whole graph.
//!   4. **Critic** (standard tier) checks the answer against the evidence,
//!      enforces the language and trims unsupported claims; skipped under
//!      budget pressure.
//! Every step emits bus events so the GUI console shows what is happening, and
//! the final answer is remembered in project memory.

use crate::bus;
use crate::llm::{prep, Complexity, LlmRequest, LlmRouter};
use crate::memory;
use anyhow::Result;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Tool registry
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String, // retrieval | graph | memory | transform | agent | action
    pub args: serde_json::Value,
}

fn tool(id: &str, name: &str, desc: &str, kind: &str, args: serde_json::Value) -> Tool {
    Tool { id: id.into(), name: name.into(), description: desc.into(), kind: kind.into(), args }
}

/// Tools available right now: built-ins + every enabled transform + agents.
pub fn registry(domain: &str) -> Vec<Tool> {
    let mut v = vec![
        tool("memory.recall", "Recall project memory", "Hybrid retrieval (grep + BM25 + vectors) over entities, relationships, notes, briefs and remembered facts.", "retrieval", serde_json::json!({"query":"string","k":"int"})),
        tool("memory.grep", "Grep exact selector", "Exact substring scan for IPs, domains, hashes, e-mails, IDs.", "retrieval", serde_json::json!({"pattern":"string"})),
        tool("graph.neighbors", "Graph neighbourhood", "Entities directly linked to an entity (by label).", "graph", serde_json::json!({"label":"string"})),
        tool("graph.filter", "Filter entities", "Entities by kind / minimum risk / tag.", "graph", serde_json::json!({"kind":"string?","min_risk":"float?","tag":"string?"})),
        tool("graph.stats", "Graph statistics", "Counts by kind, risk bands, hubs and isolated entities.", "graph", serde_json::json!({})),
        tool("memory.remember", "Remember a fact", "Store a fact/hypothesis in project memory.", "memory", serde_json::json!({"kind":"fact|hypothesis|note","text":"string"})),
    ];
    for t in crate::transforms::list().into_iter().filter(|t| t.enabled) {
        v.push(tool(&format!("transform:{}", t.id), &t.name, &format!("{} (inputs: {})", t.description, if t.input_kinds.is_empty() { "any".to_string() } else { t.input_kinds.join(", ") }), "transform", serde_json::json!({"label":"string","kind":"string"})));
    }
    for a in crate::agentlib::library(domain, "").into_iter().take(40) {
        v.push(tool(&format!("agent:{}", a.id), &a.name, &a.description, "agent", serde_json::json!({})));
    }
    v
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Observation {
    pub tool: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entity_ids: Vec<String>,
}

fn node_by_label<'a>(graph: &'a serde_json::Value, label: &str) -> Option<&'a serde_json::Value> {
    let low = label.to_lowercase();
    graph.get("nodes")?.as_array()?.iter().find(|n| n.get("label").and_then(|l| l.as_str()).map(|l| l.to_lowercase() == low).unwrap_or(false))
        .or_else(|| graph.get("nodes")?.as_array()?.iter().find(|n| n.get("label").and_then(|l| l.as_str()).map(|l| l.to_lowercase().contains(&low)).unwrap_or(false)))
}

pub fn run_tool(project: &str, tool: &str, args: &serde_json::Value, graph: &serde_json::Value) -> Observation {
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    match tool {
        "memory.recall" => {
            let q = s("query");
            let hits = memory::search(project, &q, args.get("k").and_then(|v| v.as_u64()).unwrap_or(8) as usize, Some(graph), "hybrid");
            let ids: Vec<String> = hits.iter().flat_map(|h| h.entity_ids.clone()).collect();
            Observation { tool: tool.into(), summary: hits.iter().map(|h| format!("[{}] {}: {}", h.kind, h.title, h.text.chars().take(200).collect::<String>())).collect::<Vec<_>>().join("\n"), entity_ids: ids }
        }
        "memory.grep" => {
            let hits = memory::search(project, &s("pattern"), 12, Some(graph), "grep");
            let ids: Vec<String> = hits.iter().flat_map(|h| h.entity_ids.clone()).collect();
            Observation { tool: tool.into(), summary: hits.iter().map(|h| format!("{} [{}]", h.title, h.kind)).collect::<Vec<_>>().join("; "), entity_ids: ids }
        }
        "graph.neighbors" => {
            let Some(n) = node_by_label(graph, &s("label")) else { return Observation { tool: tool.into(), summary: "entity not found".into(), entity_ids: vec![] } };
            let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let nodes = graph.get("nodes").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let lbl = |x: &str| nodes.iter().find(|m| m.get("id").and_then(|v| v.as_str()) == Some(x)).map(|m| format!("{} [{}]", m.get("label").and_then(|v| v.as_str()).unwrap_or("?"), m.get("kind").and_then(|v| v.as_str()).unwrap_or("?"))).unwrap_or(x.to_string());
            let mut ids = vec![id.clone()];
            let mut lines = Vec::new();
            for e in graph.get("edges").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                let src = e.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let tgt = e.get("target").and_then(|v| v.as_str()).unwrap_or("");
                let ty = e.get("type").and_then(|v| v.as_str()).unwrap_or("related").replace('_', " ");
                if src == id { lines.push(format!("→ {ty} → {}", lbl(tgt))); ids.push(tgt.to_string()); }
                else if tgt == id { lines.push(format!("← {ty} ← {}", lbl(src))); ids.push(src.to_string()); }
                if lines.len() > 40 { break; }
            }
            Observation { tool: tool.into(), summary: format!("{}: {}", lbl(&id), lines.join("; ")), entity_ids: ids }
        }
        "graph.filter" => {
            let kind = s("kind"); let tag = s("tag"); let min = args.get("min_risk").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mut ids = Vec::new(); let mut lines = Vec::new();
            for n in graph.get("nodes").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                let k = n.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                let r = n.get("risk").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let tags = n.get("tags").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>()).unwrap_or_default();
                if (!kind.is_empty() && k != kind) || r < min || (!tag.is_empty() && !tags.iter().any(|t| t.contains(tag.as_str()))) { continue; }
                ids.push(n.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string());
                lines.push(format!("{} [{}] {:.2}", n.get("label").and_then(|v| v.as_str()).unwrap_or("?"), k, r));
                if lines.len() >= 60 { break; }
            }
            Observation { tool: tool.into(), summary: format!("{} match(es): {}", ids.len(), lines.join("; ")), entity_ids: ids }
        }
        "graph.stats" => {
            let nodes = graph.get("nodes").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let edges = graph.get("edges").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let mut kinds: std::collections::HashMap<String, usize> = Default::default();
            let mut bands: std::collections::HashMap<String, usize> = Default::default();
            let mut deg: std::collections::HashMap<String, usize> = Default::default();
            for n in &nodes { *kinds.entry(n.get("kind").and_then(|v| v.as_str()).unwrap_or("?").into()).or_default() += 1; let r = n.get("risk").and_then(|v| v.as_f64()).unwrap_or(0.0); *bands.entry(if r >= 0.85 { "critical" } else if r >= 0.6 { "high" } else if r >= 0.35 { "medium" } else { "low" }.into()).or_default() += 1; }
            for e in &edges { for k in ["source", "target"] { if let Some(id) = e.get(k).and_then(|v| v.as_str()) { *deg.entry(id.into()).or_default() += 1; } } }
            let mut hubs: Vec<(String, usize)> = deg.iter().map(|(k, v)| (k.clone(), *v)).collect(); hubs.sort_by(|a, b| b.1.cmp(&a.1)); hubs.truncate(5);
            let lbl = |x: &str| nodes.iter().find(|m| m.get("id").and_then(|v| v.as_str()) == Some(x)).and_then(|m| m.get("label").and_then(|v| v.as_str())).unwrap_or(x).to_string();
            Observation { tool: tool.into(), summary: format!("{} nodes, {} edges; kinds {:?}; risk {:?}; hubs {}", nodes.len(), edges.len(), kinds, bands, hubs.iter().map(|(k, d)| format!("{}({})", lbl(k), d)).collect::<Vec<_>>().join(", ")), entity_ids: hubs.into_iter().map(|h| h.0).collect() }
        }
        "memory.remember" => {
            let f = memory::remember(project, &s("kind"), &s("text"), "planner", &[]);
            Observation { tool: tool.into(), summary: f.map(|f| format!("remembered {}", f.id)).unwrap_or_else(|| "nothing stored".into()), entity_ids: vec![] }
        }
        t if t.starts_with("transform:") => {
            let id = &t[10..];
            let Some(n) = node_by_label(graph, &s("label")) else { return Observation { tool: tool.into(), summary: "entity not found".into(), entity_ids: vec![] } };
            let input = serde_json::json!({"kind": n.get("kind"), "label": n.get("label"), "attributes": n.get("attributes")});
            match crate::transforms::run(id, input, serde_json::json!({})) {
                Ok(res) => Observation { tool: tool.into(), summary: prep::compact(&res.to_string(), 1500), entity_ids: vec![] },
                Err(e) => Observation { tool: tool.into(), summary: format!("transform failed: {e}"), entity_ids: vec![] },
            }
        }
        other => Observation { tool: other.into(), summary: "unknown tool".into(), entity_ids: vec![] },
    }
}

// ---------------------------------------------------------------------------
// Planner → Worker → Answer → Critic
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub tool: String,
    pub args: serde_json::Value,
}

fn deterministic_plan(question: &str) -> Vec<Step> {
    let mut v = vec![Step { tool: "memory.recall".into(), args: serde_json::json!({"query": question, "k": 8}) }];
    // selectors in the question → exact grep
    for w in question.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '@')) {
        if w.len() >= 5 && (w.contains('.') || w.contains('@')) {
            v.push(Step { tool: "memory.grep".into(), args: serde_json::json!({"pattern": w}) });
        }
    }
    let low = question.to_lowercase();
    if low.contains("risco") || low.contains("risk") || low.contains("crític") || low.contains("critical") {
        v.push(Step { tool: "graph.filter".into(), args: serde_json::json!({"min_risk": 0.6}) });
    }
    if low.contains("quantos") || low.contains("how many") || low.contains("resumo") || low.contains("summar") || low.contains("overview") || low.contains("visão geral") {
        v.push(Step { tool: "graph.stats".into(), args: serde_json::json!({}) });
    }
    v.truncate(5);
    v
}

fn plan_with_llm(router: &LlmRouter, question: &str, tools: &[Tool], graph_summary: &str, lang: &str) -> Option<Vec<Step>> {
    let tool_list = tools.iter().take(30).map(|t| format!("- {} — {} args={}", t.id, t.description.chars().take(90).collect::<String>(), t.args)).collect::<Vec<_>>().join("\n");
    let req = LlmRequest::new(
        "You are the PLANNER of an intelligence copilot. Choose at most 4 tool calls that gather the evidence needed to answer the analyst. Prefer retrieval over transforms (transforms call external services — only when the question needs new external data). Output JSON only.",
        format!("QUESTION:\n{question}\n\nGRAPH SUMMARY:\n{graph_summary}\n\nTOOLS:\n{tool_list}\n\nReturn: {{\"steps\":[{{\"tool\":\"<id>\",\"args\":{{}}}}],\"answer_style\":\"short|detailed\"}}"),
    ).label("planner").json(serde_json::json!({"type":"object"})).complexity(Complexity::Simple).fast().lang(lang);
    let resp = router.complete(&req).ok()?;
    let v = resp.as_json().ok()?;
    let steps: Vec<Step> = v.get("steps")?.as_array()?.iter().filter_map(|s| Some(Step { tool: s.get("tool")?.as_str()?.to_string(), args: s.get("args").cloned().unwrap_or(serde_json::json!({})) })).collect();
    if steps.is_empty() { None } else { Some(steps) }
}

#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub plan: Vec<Step>,
    pub planner: String,
    pub observations: Vec<Observation>,
    pub retrieval_chars: usize,
    pub critic: serde_json::Value,
    pub provider: String,
    pub model: String,
    pub cache: bool,
}

/// Full orchestrated answer. `answer_fn` builds the final answer request from
/// (question, context) — the caller owns the persona/schema.
pub fn ask(
    router: &LlmRouter,
    project: &str,
    domain: &str,
    question: &str,
    graph: &serde_json::Value,
    lang: &str,
    answer_fn: &dyn Fn(&str, &str) -> LlmRequest,
) -> Result<(serde_json::Value, Trace)> {
    let pressure = crate::llm::governor::pressure();
    let tools = registry(domain);
    bus::emit("plan.start", format!("{} tools available · pressure {:.0}%", tools.len(), pressure * 100.0));
    // graph summary for the planner (tiny)
    let nodes = graph.get("nodes").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let edges = graph.get("edges").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    for n in graph.get("nodes").and_then(|v| v.as_array()).cloned().unwrap_or_default() { *kinds.entry(n.get("kind").and_then(|v| v.as_str()).unwrap_or("?").into()).or_default() += 1; }
    let summary = format!("{nodes} entities, {edges} relationships; kinds: {}", kinds.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", "));

    // 1. plan
    let (plan, planner) = if pressure < 0.85 && nodes > 0 {
        match plan_with_llm(router, question, &tools, &summary, lang) { Some(p) => (p, "llm".to_string()), None => (deterministic_plan(question), "deterministic".to_string()) }
    } else { (deterministic_plan(question), "deterministic".to_string()) };
    bus::emit("plan.ready", format!("{} step(s) via {}: {}", plan.len(), planner, plan.iter().map(|s| s.tool.clone()).collect::<Vec<_>>().join(", ")));

    // 2. work
    let mut observations: Vec<Observation> = Vec::new();
    let mut involved: Vec<String> = Vec::new();
    for (i, step) in plan.iter().enumerate() {
        let started = std::time::Instant::now();
        let mut obs = run_tool(project, &step.tool, &step.args, graph);
        obs.summary = prep::compact(&obs.summary, 1800);
        bus::emit("worker.tool", format!("{}/{} {} → {} chars in {:.0}ms", i + 1, plan.len(), step.tool, obs.summary.len(), started.elapsed().as_secs_f32() * 1000.0));
        for id in &obs.entity_ids { if !involved.contains(id) { involved.push(id.clone()); } }
        observations.push(obs);
    }

    // 3. context (GraphRAG) + answer
    let budget = if pressure > 0.8 { 6000 } else if pressure > 0.5 { 12000 } else { 20000 };
    let (ctx, ctx_ids) = memory::build_context(project, question, Some(graph), budget);
    for id in ctx_ids { if !involved.contains(&id) { involved.push(id); } }
    let obs_text = observations.iter().filter(|o| !o.summary.is_empty()).map(|o| format!("## {}\n{}", o.tool, o.summary)).collect::<Vec<_>>().join("\n\n");
    let mut context = String::new();
    if !ctx.is_empty() { context.push_str(&ctx); context.push_str("\n\n"); }
    if !obs_text.is_empty() { context.push_str("OBSERVAÇÕES DAS FERRAMENTAS:\n"); context.push_str(&obs_text); }
    if context.trim().is_empty() {
        // no project index (e.g. mock) → fall back to a compact graph listing
        context = compact_graph_listing(graph);
    }
    let retrieval_chars = context.chars().count();
    let req = answer_fn(question, &context).lang(lang);
    let resp = router.complete(&req)?;
    let mut answer = resp.as_json().unwrap_or_else(|_| serde_json::json!({ "answer": resp.text }));

    // 4. critic
    let mut critic_v = serde_json::json!({"skipped": true});
    if pressure < 0.8 && nodes > 0 {
        let creq = LlmRequest::new(
            "You are the CRITIC. Verify the draft answer strictly against the EVIDENCE. Remove or flag claims not supported by the evidence, keep entity identifiers verbatim, enforce the requested language, and keep only what the analyst needs. Output JSON only: {\"ok\":bool,\"issues\":[\"..\"],\"answer\":\"<revised answer in the requested language, or the same if fine>\",\"confidence\":\"low|medium|high\"}",
            format!("QUESTION:\n{question}\n\nEVIDENCE:\n{}\n\nDRAFT ANSWER (JSON):\n{}", prep::compact(&context, 9000), prep::compact(&answer.to_string(), 5000)),
        ).label("critic").json(serde_json::json!({"type":"object"})).complexity(Complexity::Standard).fast().lang(lang);
        match router.complete(&creq).ok().and_then(|r| r.as_json().ok()) {
            Some(c) => {
                let ok = c.get("ok").and_then(|v| v.as_bool()).unwrap_or(true);
                let issues = c.get("issues").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                bus::emit("critic.verdict", if ok { format!("ok · {issues} note(s)") } else { format!("revised · {issues} issue(s)") });
                if let Some(rev) = c.get("answer").and_then(|v| v.as_str()).filter(|s| s.trim().len() > 20) {
                    if !ok { answer["answer"] = serde_json::Value::String(rev.to_string()); }
                }
                if let Some(conf) = c.get("confidence").cloned() { if answer.get("confidence").is_none() { answer["confidence"] = conf; } }
                critic_v = c;
            }
            None => { bus::emit("critic.skip", "critic unavailable"); }
        }
    }

    // focus: make sure the UI gets the involved entity ids too
    if !involved.is_empty() {
        let f = answer.get("focus").cloned().unwrap_or(serde_json::json!({"action":"highlight"}));
        let mut f = if f.is_object() { f } else { serde_json::json!({"action":"highlight"}) };
        f["ids"] = serde_json::Value::Array(involved.iter().take(60).map(|s| serde_json::Value::String(s.clone())).collect());
        answer["focus"] = f;
    }
    // remember the distilled answer
    if let Some(a) = answer.get("answer").and_then(|v| v.as_str()) {
        let _ = memory::remember(project, "answer", &format!("Q: {} — A: {}", question.chars().take(160).collect::<String>(), a.chars().take(700).collect::<String>()), "copilot", &involved.iter().take(20).cloned().collect::<Vec<_>>());
    }
    let trace = Trace { plan, planner, observations, retrieval_chars, critic: critic_v, provider: resp.provider.clone(), model: resp.model.clone(), cache: false };
    Ok((answer, trace))
}

fn compact_graph_listing(graph: &serde_json::Value) -> String {
    let nodes = graph.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();
    let edges = graph.get("edges").and_then(|e| e.as_array()).cloned().unwrap_or_default();
    let mut s = format!("nodes={} edges={}\n\nENTITIES:\n", nodes.len(), edges.len());
    for n in nodes.iter().take(120) {
        s.push_str(&format!("- [{}] {} (risk {:.2})\n", n.get("kind").and_then(|k| k.as_str()).unwrap_or("?"), n.get("label").and_then(|l| l.as_str()).unwrap_or("?"), n.get("risk").and_then(|r| r.as_f64()).unwrap_or(0.0)));
    }
    s.push_str("\nRELATIONSHIPS:\n");
    let lbl = |x: &str| nodes.iter().find(|m| m.get("id").and_then(|v| v.as_str()) == Some(x)).and_then(|m| m.get("label").and_then(|v| v.as_str())).unwrap_or(x).to_string();
    for e in edges.iter().take(160) {
        s.push_str(&format!("- {} --{}--> {}\n", lbl(e.get("source").and_then(|x| x.as_str()).unwrap_or("?")), e.get("type").and_then(|x| x.as_str()).unwrap_or("?"), lbl(e.get("target").and_then(|x| x.as_str()).unwrap_or("?"))));
    }
    s
}
