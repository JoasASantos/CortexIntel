//! CortexIntel engine as a library, so both the `cortex` CLI and the Tauri
//! desktop GUI drive the same code. See `api` for the high-level entry points
//! the GUI calls over Tauri commands.

pub mod agents;
pub mod audit;
pub mod auth;
pub mod cli;
pub mod config;
pub mod connectors;
pub mod correlation;
pub mod extract;
pub mod keys;
pub mod llm;
pub mod ontology;
pub mod pipeline;
pub mod plugins;
pub mod projects;
pub mod prompts;
pub mod report;
pub mod reportpdf;
pub mod risk;
pub mod serve;
pub mod sources;
pub mod store;
pub mod transforms;

/// High-level, serde-friendly API used by the GUI backend.
pub mod api {
    use crate::agents;
    use crate::config::{DataType, Domain, ProviderChoice, RunConfig};
    use crate::llm::LlmRouter;
    use crate::pipeline;
    use crate::sources;
    use anyhow::{anyhow, Result};
    use serde::Serialize;
    use std::path::PathBuf;

    /// A vertical option for the GUI menu.
    #[derive(Serialize)]
    pub struct DomainInfo {
        pub slug: String,
        pub title: String,
        pub mission: String,
    }

    /// A data-type option for the GUI menu.
    #[derive(Serialize)]
    pub struct DataTypeInfo {
        pub slug: String,
        pub category: String,
    }

    /// An agent card for the GUI.
    #[derive(Serialize)]
    pub struct AgentInfo {
        pub id: String,
        pub name: String,
        pub stage: String,
        pub mission: String,
        pub specialist: bool,
    }

    /// A backend health row for the GUI's Data Sources / Settings screen.
    #[derive(Serialize)]
    pub struct HealthInfo {
        pub name: String,
        pub ok: bool,
        pub detail: String,
    }

    pub fn list_domains() -> Vec<DomainInfo> {
        Domain::all()
            .iter()
            .map(|d| DomainInfo {
                slug: d.slug().to_string(),
                title: d.title().to_string(),
                mission: d.mission().to_string(),
            })
            .collect()
    }

    pub fn list_data_types() -> Vec<DataTypeInfo> {
        DataType::all()
            .iter()
            .map(|t| DataTypeInfo { slug: t.slug().to_string(), category: t.category().to_string() })
            .collect()
    }

    pub fn list_agents(domain_slug: &str) -> Vec<AgentInfo> {
        let domain = parse_domain(domain_slug);
        agents::catalog(domain)
            .into_iter()
            .map(|c| AgentInfo {
                id: c.id,
                name: c.name,
                stage: c.stage.as_str().to_string(),
                mission: c.mission,
                specialist: c.domain_specialized,
            })
            .collect()
    }

    pub fn doctor() -> Vec<HealthInfo> {
        let router = LlmRouter::new(ProviderChoice::Auto, None, None, false);
        router
            .health_report()
            .into_iter()
            .map(|(name, res)| match res {
                Ok(v) => HealthInfo { name, ok: true, detail: v },
                Err(e) => HealthInfo { name, ok: false, detail: e.to_string() },
            })
            .collect()
    }

    /// Parameters the GUI sends to launch a run.
    #[derive(serde::Deserialize)]
    pub struct RunParams {
        #[serde(default)]
        pub inputs: Vec<String>,
        #[serde(default = "default_domain")]
        pub domain: String,
        #[serde(default, alias = "dataType")]
        pub data_type: Option<String>,
        #[serde(default = "default_provider")]
        pub provider: String,
        #[serde(default, alias = "outputDir")]
        pub output_dir: Option<String>,
        #[serde(default)]
        pub offline: Option<bool>,
        #[serde(default, alias = "maxRecords")]
        pub max_records: Option<usize>,
        #[serde(default, alias = "projectId")]
        pub project_id: Option<String>,
    }

    fn default_domain() -> String { "generic".into() }
    fn default_provider() -> String { "auto".into() }

    /// Run the full pipeline and return the consolidated JSON document. When a
    /// `project_id` is given, the result and an activity entry are saved to it.
    pub fn run_analysis(params: RunParams) -> Result<serde_json::Value> {
        if params.inputs.is_empty() {
            return Err(anyhow!("no input sources provided"));
        }
        let domain = parse_domain(&params.domain);
        let data_type = params.data_type.as_deref().filter(|s| !s.is_empty()).and_then(parse_data_type);
        let provider = parse_provider(&params.provider);
        let offline = params.offline.unwrap_or(false) || provider == ProviderChoice::Mock;

        let mut cfg = RunConfig {
            domain,
            data_type,
            provider: if offline { ProviderChoice::Mock } else { provider },
            output_dir: PathBuf::from(params.output_dir.unwrap_or_else(|| "./cortex-out".into())),
            // Default cap protects the GUI from huge feeds (e.g. 100k-row CSVs).
            max_records: Some(params.max_records.unwrap_or(4000)),
            offline,
            ..Default::default()
        };
        if offline {
            cfg.provider = ProviderChoice::Mock;
        }

        let mut srcs: Vec<Box<dyn sources::DataSource>> = Vec::new();
        for path in &params.inputs {
            let p = PathBuf::from(path);
            if !p.exists() {
                return Err(anyhow!("input not found: {}", p.display()));
            }
            srcs.push(sources::source_for_path(&p, cfg.data_type)?);
        }

        let router = if offline {
            LlmRouter::offline(false)
        } else {
            LlmRouter::new(cfg.provider, None, None, false)
        };
        let result = pipeline::run(srcs, &cfg, &router)?;

        if let Some(pid) = &params.project_id {
            if !pid.is_empty() {
                let n_ent = result.get("entities").and_then(|e| e.as_object())
                    .map(|o| o.values().filter_map(|v| v.as_array().map(|a| a.len())).sum::<usize>()).unwrap_or(0);
                let n_rel = result.get("relationships").and_then(|r| r.as_array()).map(|a| a.len()).unwrap_or(0);
                let _ = crate::projects::add_activity(pid, "run",
                    &format!("Analysis: {} entities, {} relationships ({})", n_ent, n_rel, params.domain),
                    serde_json::json!({"entities": n_ent, "relationships": n_rel}));
                let _ = crate::projects::set_result(pid, result.clone());
            }
        }
        Ok(result)
    }

    /// AI copilot: answer a question about the current graph, optionally
    /// proposing new entities/relationships to expand the investigation.
    #[derive(serde::Deserialize)]
    pub struct AskParams {
        pub question: String,
        #[serde(default = "default_domain")]
        pub domain: String,
        #[serde(default = "default_provider")]
        pub provider: String,
        /// Current graph context: { nodes: [...], edges: [...] }.
        #[serde(default)]
        pub graph: serde_json::Value,
        /// Optional project AI instructions to steer the answer.
        #[serde(default, alias = "aiInstructions")]
        pub ai_instructions: Option<String>,
    }

    pub fn ask(params: AskParams) -> Result<serde_json::Value> {
        if params.question.trim().is_empty() {
            return Err(anyhow!("empty question"));
        }
        let domain = parse_domain(&params.domain);
        let provider = parse_provider(&params.provider);
        let router = if provider == ProviderChoice::Mock {
            LlmRouter::offline(false)
        } else {
            LlmRouter::new(provider, None, None, false)
        };
        let mut context = summarize_graph_for_prompt(&params.graph);
        if let Some(instr) = params.ai_instructions.as_deref().filter(|s| !s.trim().is_empty()) {
            context = format!("OPERATOR INSTRUCTIONS (steer your analysis):\n{instr}\n\n{context}");
        }
        let req = crate::agents::ask_request(domain, &params.question, &context);
        let resp = router.complete(&req)?;
        resp.as_json().or_else(|_| Ok(serde_json::json!({ "answer": resp.text })))
    }

    /// Compact the frontend graph (nodes/edges) into a prompt-sized context.
    fn summarize_graph_for_prompt(graph: &serde_json::Value) -> String {
        let nodes = graph.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();
        let edges = graph.get("edges").and_then(|e| e.as_array()).cloned().unwrap_or_default();
        let mut s = format!("nodes={} edges={}\n\nENTITIES:\n", nodes.len(), edges.len());
        for n in nodes.iter().take(150) {
            let kind = n.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
            let label = n.get("label").and_then(|l| l.as_str()).unwrap_or("?");
            let risk = n.get("risk").and_then(|r| r.as_f64()).or_else(|| n.get("risk_score").and_then(|r| r.as_f64())).unwrap_or(0.0);
            s.push_str(&format!("- [{kind}] {label} (risk {risk:.2})\n"));
        }
        s.push_str("\nRELATIONSHIPS:\n");
        for e in edges.iter().take(200) {
            let src = e.get("source").or_else(|| e.get("source_id")).and_then(|x| x.as_str()).unwrap_or("?");
            let tgt = e.get("target").or_else(|| e.get("target_id")).and_then(|x| x.as_str()).unwrap_or("?");
            let rel = e.get("type").or_else(|| e.get("rel_type")).and_then(|x| x.as_str()).unwrap_or("?");
            s.push_str(&format!("- {src} --{rel}--> {tgt}\n"));
        }
        s
    }

    /// Instance config (country for locale-aware KYC + onboarding state).
    pub fn get_config() -> serde_json::Value {
        let s = crate::store::get_settings();
        serde_json::json!({ "country": s.country, "onboarded": s.onboarded, "supported": ["BR", "US"] })
    }

    pub fn set_config(country: &str, onboarded: bool) -> Result<serde_json::Value> {
        let mut s = crate::store::get_settings();
        if !country.is_empty() {
            s.country = country.to_uppercase();
        }
        s.onboarded = onboarded || s.onboarded;
        crate::store::save_settings(&s)?;
        Ok(get_config())
    }

    /// Save uploaded file bytes to the uploads dir and return its path (lets the
    /// browser "select a file from the PC" and feed it to the pipeline).
    pub fn save_upload(filename: &str, bytes: &[u8]) -> Result<String> {
        let dir = crate::store::uploads_dir();
        crate::store::ensure_dir(&dir)?;
        let safe: String = filename.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_').collect();
        let name = if safe.is_empty() { "upload.dat".into() } else { safe };
        let path = dir.join(format!("{}-{}", uuid::Uuid::new_v4().simple(), name));
        std::fs::write(&path, bytes)?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Test a connector (db/bigquery/datalake). Returns a status string.
    pub fn connector_test(kind: &str, cfg: &serde_json::Value) -> Result<String> {
        crate::connectors::test(kind, cfg)
    }

    /// Fetch data through a connector and run the pipeline over it.
    #[derive(serde::Deserialize)]
    pub struct ConnectorRunParams {
        pub kind: String,
        #[serde(default)]
        pub config: serde_json::Value,
        #[serde(default = "default_domain")]
        pub domain: String,
        #[serde(default = "default_provider")]
        pub provider: String,
        #[serde(default, alias = "projectId")]
        pub project_id: Option<String>,
        #[serde(default, alias = "maxRecords")]
        pub max_records: Option<usize>,
    }

    pub fn connector_run(p: ConnectorRunParams) -> Result<serde_json::Value> {
        let path = crate::connectors::fetch(&p.kind, &p.config)?;
        run_analysis(RunParams {
            inputs: vec![path.to_string_lossy().to_string()],
            domain: p.domain,
            data_type: None,
            provider: p.provider,
            output_dir: None,
            offline: None,
            max_records: p.max_records,
            project_id: p.project_id,
        })
    }

    /// Render a project's consolidated analysis to a PDF (via Typst) and return
    /// the PDF path.
    pub fn report_pdf(project_id: &str) -> Result<serde_json::Value> {
        let p = crate::projects::load(project_id)?;
        let c = p.last_result.ok_or_else(|| anyhow!("project has no analysis yet — run one first"))?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
        let path = crate::reportpdf::to_pdf(&c, &p.name, &p.domain, &p.owner, &now)?;
        Ok(serde_json::json!({ "path": path }))
    }

    /// Load a previously written graph.json from an output directory.
    pub fn load_graph(dir: &str) -> Result<serde_json::Value> {
        let path = PathBuf::from(dir).join("graph.json");
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow!("cannot read {}: {e}", path.display()))?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn parse_domain(s: &str) -> Domain {
        Domain::all()
            .iter()
            .copied()
            .find(|d| d.slug() == s)
            .unwrap_or(Domain::Generic)
    }

    fn parse_data_type(s: &str) -> Option<DataType> {
        DataType::all().iter().copied().find(|t| t.slug() == s)
    }

    fn parse_provider(s: &str) -> ProviderChoice {
        match s {
            "claude" => ProviderChoice::Claude,
            "codex" => ProviderChoice::Codex,
            "mock" => ProviderChoice::Mock,
            _ => ProviderChoice::Auto,
        }
    }
}
