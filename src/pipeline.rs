//! The intelligence pipeline. Implements the operator flow:
//!
//!   Secure ingestion → Normalization & dedup → Entities → Graph correlation →
//!   Risk prioritization → Investigation/protection/evidence → Audit, retention
//!   & legal disposal.
//!
//! Each stage has a deterministic core and an optional LLM-agent augmentation
//! routed through [`LlmRouter`]. Nothing is persisted to a database — the run
//! reads records in, builds an in-memory graph, and writes JSON/Markdown out.

use crate::agents;
use crate::audit::{AuditLog, RetentionPolicy};
use crate::config::{DataType, RunConfig};
use crate::correlation;
use crate::extract;
use crate::llm::LlmRouter;
use crate::ontology::{Entity, EntityKind, KnowledgeGraph, RiskBand, Relationship};
use crate::report::RunOutput;
use crate::risk;
use crate::sources::{DataSource, McpSource, RecordBatch};
use anyhow::Result;
use chrono::Utc;
use owo_colors::OwoColorize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Run the full pipeline over one or more sources.
pub fn run(
    sources: Vec<Box<dyn DataSource>>,
    config: &RunConfig,
    router: &LlmRouter,
) -> Result<serde_json::Value> {
    let started_at = Utc::now();
    let mut audit = AuditLog::new(&config.output_dir, &config.operator, &config.legal_basis)?;
    let mut graph = KnowledgeGraph::new();

    step("1/7", "Secure ingestion");
    let mut batches: Vec<RecordBatch> = Vec::new();
    for src in &sources {
        // MCP sources produce a fetch plan; surface it and let the agent runtime
        // execute it out-of-band. Here we ingest whatever the connector returns.
        let batch = src.load()?;
        if config.verbose {
            eprintln!("  · loaded {} records from {}", batch.records.len(), src.describe());
        }
        if src.describe().starts_with("mcp") {
            note_mcp_plan(src);
        }
        batches.push(batch);
    }
    let total_records: usize = batches.iter().map(|b| b.records.len()).sum();
    audit.record(
        Utc::now(),
        "ingest_records",
        "ingestion",
        &format!("{total_records} records from {} source(s)", sources.len()),
        "operator-initiated run",
        false,
        false,
        None,
        None,
    );
    println!("      ingested {} record(s)", total_records);

    step("2/7", "Classification");
    let declared = config.data_type.or_else(|| batches.iter().find_map(|b| b.declared_type));
    let data_type = match declared {
        Some(dt) => dt,
        None => classify(&batches, config, router, &mut audit),
    };
    println!("      data type → {}", data_type.slug().bright_white());

    step("3/7", "Normalization, dedup & entity extraction");
    // Load active plugins for this vertical (extra field mappings + risk signals).
    let plugins = crate::plugins::active_for(config.domain.slug());
    let extra_mappings: Vec<(String, EntityKind)> = plugins
        .iter()
        .flat_map(|p| p.field_mappings.iter())
        .map(|m| (m.field.clone(), EntityKind::parse(&m.kind)))
        .collect();
    let mut extra_signals: Vec<(String, f32)> = plugins
        .iter()
        .flat_map(|p| p.risk_signals.iter())
        .map(|r| (r.token.to_lowercase(), r.weight))
        .collect();
    if !plugins.is_empty() {
        println!("      {} plugin(s) active: {}", plugins.len(), plugins.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "));
    }

    // Auto-ontology: profile columns of the first batch and fold the discovered
    // column→entity mappings into the extractor, so undeclared feeds still build
    // a graph without any schema flag. Deterministic; carries provenance.
    let mut auto_mappings: Vec<(String, EntityKind)> = Vec::new();
    if let Some(first) = batches.first() {
        let profile = crate::profile::profile_batch(first);
        auto_mappings = profile.entity_mappings();
        if !auto_mappings.is_empty() {
            let cols: Vec<String> = profile
                .columns
                .iter()
                .filter(|c| c.entity_kind.is_some())
                .map(|c| format!("{}→{} ({:.0}%)", c.name, c.semantic.as_str(), c.confidence * 100.0))
                .collect();
            println!("      auto-ontology: {}", cols.join(", "));
        }
    }
    // Combine plugin + auto mappings (plugin wins on conflict via ordering).
    let mut all_mappings = auto_mappings;
    for m in &extra_mappings {
        if !all_mappings.iter().any(|(f, _)| f == &m.0) {
            all_mappings.push(m.clone());
        }
    }
    let _ = &mut extra_signals;

    let cap = config.max_records;
    let mut ingested = 0usize;
    let mut sensitive_seen = false;
    'outer: for batch in &batches {
        for rec in &batch.records {
            if let Some(max) = cap {
                if ingested >= max {
                    println!("      {} record cap reached — {} of {} processed", max, ingested, total_records);
                    break 'outer;
                }
            }
            ingested += 1;
            let res = extract::extract_record(rec, data_type, &all_mappings);
            // Upsert entities and build a per-record label→id resolution table.
            let mut local: HashMap<String, String> = HashMap::new();
            for e in res.entities {
                if e.sensitive {
                    sensitive_seen = true;
                }
                let label = e.label.clone();
                let id = graph.upsert_entity(e);
                local.insert(crate::ontology::normalize_key(&label), id);
            }
            for link in res.links {
                let s = local.get(&crate::ontology::normalize_key(&link.source_label));
                let t = local.get(&crate::ontology::normalize_key(&link.target_label));
                if let (Some(s), Some(t)) = (s, t) {
                    graph.add_relationship(Relationship::new(
                        s.clone(),
                        link.rel_type,
                        t.clone(),
                        link.confidence,
                    ));
                }
            }
        }
    }
    // LLM augmentation over a bounded sample.
    llm_extract_augment(&batches, data_type, config, router, &mut graph);
    audit.record(
        Utc::now(),
        "extract_entities",
        "extraction",
        &format!("{} entities, {} relationships", graph.entity_count(), graph.relationship_count()),
        "entity resolution",
        sensitive_seen,
        false,
        None,
        None,
    );
    println!(
        "      {} entities · {} relationships",
        graph.entity_count().to_string().green(),
        graph.relationship_count().to_string().green()
    );

    // "Potentiate": derive normalized/derived attributes and hub entities so the
    // correlator has more to link (e.g. the registrable domain of every URL).
    // CORTEX_NO_ENRICH=1 skips it (A/B comparison + escape hatch).
    let enr = if std::env::var("CORTEX_NO_ENRICH").is_ok() {
        crate::enrich::EnrichStats::default()
    } else {
        crate::enrich::enrich(&mut graph)
    };
    if enr.attrs + enr.entities + enr.edges + enr.ref_matches > 0 {
        audit.record(
            Utc::now(),
            "enrich_entities",
            "enrichment",
            &format!("{} derived attributes, {} derived entities, {} derived links, {} reference-source matches", enr.attrs, enr.entities, enr.edges, enr.ref_matches),
            "data potentiation",
            false,
            false,
            None,
            None,
        );
        println!(
            "      +{} derived attributes · +{} derived entities · +{} derived links",
            enr.attrs.to_string().green(),
            enr.entities.to_string().green(),
            enr.edges.to_string().green()
        );
        if enr.ref_matches > 0 {
            println!("      ⚑ {} reference-source match(es) (known-hash feed)", enr.ref_matches.to_string().red());
        }
    }

    step("4/7", "Graph correlation");
    let added = correlation::correlate(&mut graph);
    // LLM correlation augmentation.
    let llm_added = llm_correlate(&graph_snapshot_entities(&graph), config, router)
        .map(|v| correlation::merge_llm(&mut graph, &v))
        .unwrap_or(0);
    audit.record(
        Utc::now(),
        "correlate_graph",
        "correlation",
        &format!("{} heuristic + {} llm correlation edges", added, llm_added),
        "relationship discovery",
        false,
        false,
        None,
        None,
    );
    println!("      +{} correlation edges", (added + llm_added).to_string().green());

    // GEOINT: geography as a correlation signal — link entities that are
    // physically near each other (co_located_with), feeding risk/network/map.
    let geo = crate::geoint::correlate_geo(&mut graph);
    if geo.colocations > 0 {
        audit.record(
            Utc::now(),
            "geo_correlation",
            "geoint",
            &format!("{} co-location links across {} geolocated entities", geo.colocations, geo.geolocated),
            "geospatial correlation",
            false,
            false,
            None,
            None,
        );
        println!("      +{} co-location links ({} geolocated)", geo.colocations.to_string().green(), geo.geolocated);
    }

    // Robustness for messy data: fuzzy near-duplicate + temporal correlation.
    let fz = crate::fuzzy::apply(&mut graph);
    if fz.similar_links + fz.temporal_links > 0 {
        audit.record(Utc::now(), "fuzzy_correlation", "correlation",
            &format!("{} fuzzy similar links, {} temporal links", fz.similar_links, fz.temporal_links),
            "robust correlation", false, false, None, None);
        println!("      +{} fuzzy · +{} temporal links", fz.similar_links.to_string().green(), fz.temporal_links);
    }

    // Intelligence-discipline signals (HUMINT reliability grading, SIGINT comms
    // pattern, OSINT selector reuse) — deterministic, feed risk + the graph.
    let disc = crate::disciplines::apply(&mut graph);
    if disc.humint_graded + disc.sigint_links + disc.osint_links > 0 {
        audit.record(
            Utc::now(),
            "discipline_signals",
            "disciplines",
            &format!("HUMINT graded {}, SIGINT {} comms links, OSINT {} selector links", disc.humint_graded, disc.sigint_links, disc.osint_links),
            "discipline analysis",
            false,
            false,
            None,
            None,
        );
        println!("      disciplines: HUMINT {} graded · SIGINT +{} · OSINT +{}", disc.humint_graded.to_string().green(), disc.sigint_links, disc.osint_links);
    }

    // Identity resolution (E): fold same-entity aliases into canonical nodes
    // before risk/assessment, so scoring/intelligence see one entity, not three.
    let resolution = crate::resolve::resolve(&mut graph);
    if !resolution.merged.is_empty() || !resolution.suggestions.is_empty() {
        println!(
            "      identity resolution: {} merged, {} suggestion(s)",
            resolution.merged.len(),
            resolution.suggestions.len()
        );
        audit.record(
            Utc::now(),
            "resolve_identities",
            "identity-resolution",
            &format!("{} auto-merged, {} suggested", resolution.merged.len(), resolution.suggestions.len()),
            "entity resolution",
            false,
            false,
            None,
            None,
        );
    }

    // Network science: structural intelligence (broker / importance / communities).
    let net = crate::netsci::analyze(&graph);
    if !net.betweenness.is_empty() {
        for (id, b) in &net.betweenness {
            if let Some(e) = graph.entities.get_mut(id) {
                e.attributes.insert("betweenness".into(), format!("{:.3}", b));
                if let Some(p) = net.pagerank.get(id) { e.attributes.insert("pagerank".into(), format!("{:.3}", p)); }
                if let Some(c) = net.community.get(id) { e.attributes.insert("community".into(), c.to_string()); }
            }
        }
        if let Some(broker) = &net.top_broker {
            if let Some(e) = graph.entities.get_mut(broker) {
                if !e.tags.iter().any(|t| t == "broker") { e.tags.push("broker".into()); }
            }
        }
        let broker_label = net.top_broker.as_ref().and_then(|id| graph.entities.get(id)).map(|e| e.label.clone()).unwrap_or_default();
        audit.record(
            Utc::now(),
            "network_analysis",
            "network-science",
            &format!("{} communities (modularity {:.2}); top broker: {}", net.communities, net.modularity, broker_label),
            "structural analysis",
            false,
            false,
            None,
            None,
        );
        println!(
            "      {} communities (Q={:.2}) · top broker: {}",
            net.communities.to_string().green(),
            net.modularity,
            broker_label.cyan()
        );
    }

    // Anomaly detection: entities that deviate from their same-kind peers
    // (outlier degree/betweenness/pagerank) or behave oddly (off-hours activity).
    // Writes an anomaly_score that risk prioritization folds in.
    let anom = crate::anomaly::detect(&graph);
    if !anom.anomalies.is_empty() {
        for a in &anom.anomalies {
            if let Some(e) = graph.entities.get_mut(&a.entity_id) {
                e.attributes.insert("anomaly_score".into(), format!("{:.2}", a.score));
                e.attributes.insert("anomaly_reason".into(), a.reason.clone());
                if !e.tags.iter().any(|t| t == "anomaly") { e.tags.push("anomaly".into()); }
            }
        }
        let top = anom.anomalies.iter().take(1).next().and_then(|a| graph.entities.get(&a.entity_id)).map(|e| e.label.clone()).unwrap_or_default();
        audit.record(
            Utc::now(),
            "anomaly_detection",
            "anomaly",
            &format!("{} anomalous entit(y/ies); top: {}", anom.anomalies.len(), top),
            "outlier analysis",
            false,
            false,
            None,
            None,
        );
        println!("      ⚑ {} anomalous entit(y/ies) · top: {}", anom.anomalies.len().to_string().yellow(), top.cyan());
    }

    step("5/7", "Risk prioritization");
    let mut risk_report = risk::score_graph(&graph, config.domain, &extra_signals);
    if let Some(v) = llm_risk(&graph_summary(&graph, &risk_report), config, router) {
        risk::merge_llm(&mut risk_report, &v);
    }
    // Write scores back onto entities.
    for a in &risk_report.assessments {
        if let Some(e) = graph.entities.get_mut(&a.entity_id) {
            e.risk_score = Some(a.risk_score);
            e.risk_band = Some(RiskBand::from_score(a.risk_score));
        }
    }
    // Composite Intelligence Score (0–100, explainable) blending risk + network +
    // anomaly + connectivity + reference matches — one number with the "why".
    crate::iscore::compute(&mut graph);
    // Automatic tags: derived, deterministic labels that make the graph filterable
    // (email provider, risk band, hub, isolated, sensitive) without manual work.
    let deg_all = graph.degree_centrality();
    let ids: Vec<String> = graph.entities.keys().cloned().collect();
    for id in ids {
        let d = *deg_all.get(&id).unwrap_or(&0);
        let mut adds: Vec<String> = Vec::new();
        if let Some(e) = graph.entities.get(&id) {
            if let Some(b) = e.risk_band {
                if b >= RiskBand::High { adds.push(format!("risk:{}", b.as_str())); }
            }
            if e.kind == EntityKind::Account {
                if let Some((_, dom)) = e.label.split_once('@') {
                    adds.push(format!("provider:{}", dom.trim().to_lowercase()));
                }
            }
            if d >= 8 { adds.push("hub".into()); }
            if d == 0 { adds.push("isolated".into()); }
            if e.sensitive { adds.push("sensitive".into()); }
        }
        if let Some(e) = graph.entities.get_mut(&id) {
            for t in adds { if !e.tags.contains(&t) { e.tags.push(t); } }
        }
    }
    audit.record(
        Utc::now(),
        "run_ai_assessment",
        "risk",
        &format!("case risk {} ({:.2})", risk_report.case_risk_band, risk_report.case_risk_score),
        "risk scoring",
        false,
        false,
        Some(router.choice().to_string()),
        None,
    );
    println!(
        "      case risk {} ({:.2})",
        risk_report.case_risk_band.bright_white(),
        risk_report.case_risk_score
    );

    step("6/7", "Investigation, protection & evidence");
    let brief_input = investigation_input(&graph, &risk_report);
    let investigation = llm_investigate(&brief_input, config, router).unwrap_or_else(|| {
        json!({
            "summary": "No LLM narrative generated (offline or provider unavailable). See prioritized entities and recommended actions.",
            "key_findings": [],
            "next_steps": [],
        })
    });
    audit.record(
        Utc::now(),
        "generate_brief",
        "investigation",
        "investigative brief",
        "analysis synthesis",
        sensitive_seen,
        false,
        Some(router.choice().to_string()),
        None,
    );
    println!("      investigative brief generated");

    step("7/7", "Audit, retention & legal disposal");
    let retention = RetentionPolicy::new(Utc::now(), config.retention_days, &config.legal_basis);
    let run_summary = run_summary(&graph, &risk_report, &audit);
    let audit_summary = llm_audit(&run_summary, config, router).unwrap_or_else(|| {
        json!({
            "summary": "Heuristic governance summary.",
            "sensitive_entities_touched": audit.sensitive_touch_count(),
            "actions_requiring_authorization": [],
            "retention_note": retention.note,
        })
    });
    audit.record(
        Utc::now(),
        "governance_review",
        "audit",
        &format!("retention {} days", config.retention_days),
        "compliance review",
        false,
        true,
        None,
        None,
    );

    // Information → intelligence: deterministic assessment + next-best-actions.
    // Link prediction: infer likely-but-absent edges (topological, deterministic).
    // Added as `predicted_link` (marked predicted) so they're distinct from fact.
    let link_topk = std::env::var("CORTEX_LINK_TOPK").ok().and_then(|s| s.parse().ok()).unwrap_or(12usize);
    let predictions = crate::linkpred::predict(&graph, link_topk);
    if !predictions.is_empty() {
        crate::linkpred::add_to_graph(&mut graph, &predictions);
        audit.record(
            Utc::now(),
            "link_prediction",
            "inference",
            &format!("{} likely-but-absent links inferred", predictions.len()),
            "relationship inference",
            false,
            false,
            None,
            None,
        );
        println!("      +{} predicted link(s) (inferred, not observed)", predictions.len().to_string().green());
    }

    let assessment = crate::assessment::assess(&graph, &risk_report, config.domain, &config.lang);
    let next_actions = crate::assessment::next_best_actions(&graph, &risk_report, config.domain, &config.lang);

    // Threshold calibration report (opt-in) — measures anomaly / link-prediction /
    // perceptual-hash behaviour on THIS dataset and recommends threshold values.
    if std::env::var("CORTEX_CALIBRATE").is_ok() {
        crate::calibrate::report(&graph);
    }

    let finished_at = Utc::now();
    let output = RunOutput {
        config,
        graph: &graph,
        risk: &risk_report,
        investigation: &investigation,
        audit_summary: &audit_summary,
        audit: &audit,
        retention: &retention,
        assessment: &assessment,
        next_actions: &next_actions,
        resolution: &resolution,
        started_at,
        finished_at,
    };
    let consolidated = output.consolidated();
    let written = output.write_all(&config.output_dir)?;
    output.print_terminal();
    println!();
    println!("  {} {}", "output →".dimmed(), config.output_dir.display());
    for p in &written {
        println!("    · {}", p.display());
    }
    Ok(consolidated)
}

fn step(n: &str, title: &str) {
    println!("{} {}", format!("[{n}]").dimmed(), title.bold());
}

fn note_mcp_plan(src: &Box<dyn DataSource>) {
    println!(
        "      {} this MCP source declares a fetch plan; execute it via an MCP-enabled agent (Claude/Codex) and re-ingest the returned rows.",
        "note:".yellow()
    );
    let _ = src; // manifest details already surfaced as a record
}

// ---------------------------------------------------------------------------
// LLM-augmentation helpers (all optional; return None on any failure/offline).
// ---------------------------------------------------------------------------

fn classify(
    batches: &[RecordBatch],
    config: &RunConfig,
    router: &LlmRouter,
    audit: &mut AuditLog,
) -> DataType {
    let sample = batches
        .iter()
        .flat_map(|b| b.records.iter())
        .take(3)
        .map(|r| r.blob())
        .collect::<Vec<_>>()
        .join("\n---\n");
    if sample.is_empty() {
        return DataType::Generic;
    }
    let req = agents::classify_request(config.domain, &sample);
    match router.complete(&req).and_then(|r| r.as_json()) {
        Ok(v) => {
            audit.record(
                Utc::now(),
                "classify_data",
                "classification",
                "sampled records",
                "routing",
                false,
                false,
                Some(router.choice().to_string()),
                None,
            );
            v.get("data_type")
                .and_then(|d| d.as_str())
                .map(parse_data_type)
                .unwrap_or(DataType::Generic)
        }
        Err(_) => DataType::Generic,
    }
}

fn parse_data_type(s: &str) -> DataType {
    match s.trim().to_lowercase().as_str() {
        "case" => DataType::Case,
        "report" => DataType::Report,
        "media" => DataType::Media,
        "account" => DataType::Account,
        "person" => DataType::Person,
        "device" => DataType::Device,
        "network" => DataType::Network,
        "url" => DataType::Url,
        "communication" => DataType::Communication,
        "financial" => DataType::Financial,
        "location" => DataType::Location,
        _ => DataType::Generic,
    }
}

fn llm_extract_augment(
    batches: &[RecordBatch],
    data_type: DataType,
    config: &RunConfig,
    router: &LlmRouter,
    graph: &mut KnowledgeGraph,
) {
    let payload = batches
        .iter()
        .flat_map(|b| b.records.iter())
        .take(40)
        .map(|r| r.blob())
        .collect::<Vec<_>>()
        .join("\n---\n");
    if payload.trim().is_empty() {
        return;
    }
    let req = agents::extract_request(config.domain, data_type, &payload);
    let Ok(v) = router.complete(&req).and_then(|r| r.as_json()) else {
        return;
    };
    // Merge entities by label, then relationships by resolved id.
    let mut label_id: HashMap<String, String> = graph
        .entities
        .iter()
        .map(|(id, e)| (crate::ontology::normalize_key(&e.label), id.clone()))
        .collect();

    if let Some(ents) = v.get("entities").and_then(|x| x.as_array()) {
        for e in ents {
            let kind = e
                .get("kind")
                .and_then(|k| k.as_str())
                .map(EntityKind::parse)
                .unwrap_or(EntityKind::Unknown);
            let Some(label) = e.get("label").and_then(|l| l.as_str()) else {
                continue;
            };
            let mut ent = Entity::new(kind, label).with_source("llm-extractor");
            if let Some(attrs) = e.get("attributes").and_then(|a| a.as_object()) {
                for (k, val) in attrs {
                    if let Some(s) = val.as_str() {
                        ent.attributes.insert(k.clone(), s.to_string());
                    } else {
                        ent.attributes.insert(k.clone(), val.to_string());
                    }
                }
            }
            if let Some(tags) = e.get("tags").and_then(|t| t.as_array()) {
                for t in tags {
                    if let Some(s) = t.as_str() {
                        ent.tags.push(s.to_string());
                    }
                }
            }
            let key = crate::ontology::normalize_key(label);
            let id = graph.upsert_entity(ent);
            label_id.insert(key, id);
        }
    }
    if let Some(rels) = v.get("relationships").and_then(|x| x.as_array()) {
        for r in rels {
            let (Some(s), Some(rel), Some(t)) = (
                r.get("source").and_then(|x| x.as_str()),
                r.get("type").and_then(|x| x.as_str()),
                r.get("target").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            let si = label_id.get(&crate::ontology::normalize_key(s));
            let ti = label_id.get(&crate::ontology::normalize_key(t));
            if let (Some(si), Some(ti)) = (si, ti) {
                let conf = r.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.5) as f32;
                graph.add_relationship(Relationship::new(si.clone(), rel, ti.clone(), conf));
            }
        }
    }
}

fn llm_correlate(entities_json: &str, config: &RunConfig, router: &LlmRouter) -> Option<Value> {
    let req = agents::correlate_request(config.domain, entities_json);
    router.complete(&req).and_then(|r| r.as_json()).ok()
}

fn llm_risk(summary: &str, config: &RunConfig, router: &LlmRouter) -> Option<Value> {
    let req = agents::risk_request(config.domain, summary);
    router.complete(&req).and_then(|r| r.as_json()).ok()
}

fn llm_investigate(input: &str, config: &RunConfig, router: &LlmRouter) -> Option<Value> {
    let req = agents::investigate_request(config.domain, input);
    router.complete(&req).and_then(|r| r.as_json()).ok()
}

fn llm_audit(summary: &str, config: &RunConfig, router: &LlmRouter) -> Option<Value> {
    let req = agents::audit_request(config.domain, summary);
    router.complete(&req).and_then(|r| r.as_json()).ok()
}

// ---------------------------------------------------------------------------
// Payload builders for the agents (compact, id-based, no raw sensitive data).
// ---------------------------------------------------------------------------

/// Compact JSON list of entities (id, kind, label, top attrs) for correlation.
fn graph_snapshot_entities(graph: &KnowledgeGraph) -> String {
    let items: Vec<Value> = graph
        .entities
        .values()
        .take(200)
        .map(|e| {
            json!({
                "id": e.id,
                "kind": e.kind.as_str(),
                "label": e.label,
                "attributes": e.attributes,
            })
        })
        .collect();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
}

fn graph_summary(graph: &KnowledgeGraph, risk: &crate::risk::RiskReport) -> String {
    let deg = graph.degree_centrality();
    let mut lines = vec![format!(
        "entities={} relationships={}",
        graph.entity_count(),
        graph.relationship_count()
    )];
    for a in risk.assessments.iter().take(40) {
        let d = deg.get(&a.entity_id).copied().unwrap_or(0);
        lines.push(format!(
            "{} [{}] label=\"{}\" degree={} base_risk={:.2}",
            a.entity_id, a.entity_kind, a.entity_label, d, a.risk_score
        ));
    }
    lines.join("\n")
}

fn investigation_input(graph: &KnowledgeGraph, risk: &crate::risk::RiskReport) -> String {
    let mut s = format!(
        "case_risk={} ({:.2})\nentities={} relationships={}\n\nTOP ENTITIES:\n",
        risk.case_risk_band,
        risk.case_risk_score,
        graph.entity_count(),
        graph.relationship_count()
    );
    for a in risk.assessments.iter().take(15) {
        s.push_str(&format!(
            "- [{}] {} risk={:.2} action={} review={}\n",
            a.entity_kind, a.entity_label, a.risk_score, a.recommended_action, a.requires_human_review
        ));
    }
    s.push_str("\nKEY RELATIONSHIPS:\n");
    for r in graph.relationships.iter().take(40) {
        let sl = graph.entities.get(&r.source_id).map(|e| e.label.as_str()).unwrap_or("?");
        let tl = graph.entities.get(&r.target_id).map(|e| e.label.as_str()).unwrap_or("?");
        s.push_str(&format!("- {} --{}--> {} (conf {:.2})\n", sl, r.rel_type, tl, r.confidence));
    }
    s
}

fn run_summary(graph: &KnowledgeGraph, risk: &crate::risk::RiskReport, audit: &AuditLog) -> String {
    format!(
        "entities={} relationships={} case_risk={} sensitive_touches={} audit_events={}",
        graph.entity_count(),
        graph.relationship_count(),
        risk.case_risk_band,
        audit.sensitive_touch_count(),
        audit.events.len()
    )
}

/// Expose the MCP manifest reader for the CLI `sources` command.
pub fn read_mcp_manifest(path: &std::path::Path) -> Result<String> {
    let src = McpSource {
        manifest: path.to_path_buf(),
        declared: None,
    };
    let m = src.manifest()?;
    Ok(format!(
        "MCP fetch plan → server='{}' tool='{}' args={} records_path={:?}",
        m.server, m.tool, m.arguments, m.records_path
    ))
}
