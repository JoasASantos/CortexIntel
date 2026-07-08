//! Command-line surface. `cortex` drives the whole platform: run the pipeline,
//! browse the agent catalog, check the LLM backends, inspect data sources, and
//! an interactive selection menu for the "pick your vertical + data type" flow.

use crate::config::{DataType, Domain, ProviderChoice, RunConfig};
use crate::llm::LlmRouter;
use crate::{agents, pipeline, sources};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "cortex",
    version,
    about = "CortexIntel — agnostic data-collection & intelligence engine with an LLM decision layer (Claude + Codex)",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run the full ingestion → graph → risk → investigation → audit pipeline.
    Run(RunArgs),
    /// Interactive menu: pick vertical, data type, provider and sources.
    Menu,
    /// List the specialized agents for a vertical.
    Agents {
        #[arg(long, value_enum, default_value_t = Domain::Generic)]
        domain: Domain,
    },
    /// Verify the Claude and Codex CLIs are installed and authenticated.
    Doctor,
    /// Describe a data source or MCP manifest without running the pipeline.
    Sources {
        /// Path to a csv/json/mcp source to inspect.
        path: PathBuf,
    },
    /// Scaffold a sample dataset + MCP manifest to try the platform.
    Init {
        #[arg(long, default_value = "./cortex-demo")]
        dir: PathBuf,
    },
    /// Serve the desktop UI locally over HTTP (open in a normal browser).
    Serve {
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Open the default browser automatically.
        #[arg(long)]
        open: bool,
    },
    /// Launch the native macOS desktop app (Tauri), or fall back to the browser.
    Desktop {
        #[arg(long, default_value_t = 8787)]
        port: u16,
    },
}

#[derive(Parser)]
pub struct RunArgs {
    /// One or more input sources (csv/tsv/json/jsonl/ndjson or .mcp manifest).
    #[arg(long = "input", short = 'i', required = true, num_args = 1..)]
    pub input: Vec<PathBuf>,

    /// Business vertical (drives agents, prompts and risk weighting).
    #[arg(long, value_enum, default_value_t = Domain::Generic)]
    pub domain: Domain,

    /// Force a data type instead of auto-classifying.
    #[arg(long, value_enum)]
    pub data_type: Option<DataType>,

    /// Which LLM backend to use.
    #[arg(long, value_enum, default_value_t = ProviderChoice::Auto)]
    pub provider: ProviderChoice,

    /// Override the Claude model.
    #[arg(long)]
    pub claude_model: Option<String>,

    /// Override the Codex model.
    #[arg(long)]
    pub codex_model: Option<String>,

    /// Output directory.
    #[arg(long, short = 'o', default_value = "./cortex-out")]
    pub out: PathBuf,

    /// Operator identity recorded in the audit log.
    #[arg(long)]
    pub operator: Option<String>,

    /// Legal basis recorded for this run.
    #[arg(long, default_value = "internal_authorization")]
    pub legal_basis: String,

    /// Retention window in days before disposal is due.
    #[arg(long, default_value_t = 365)]
    pub retention_days: i64,

    /// Do not call any real LLM (deterministic offline run).
    #[arg(long)]
    pub offline: bool,

    /// Verbose stage logging.
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

impl RunArgs {
    fn into_config(self) -> RunConfig {
        let mut cfg = RunConfig {
            domain: self.domain,
            data_type: self.data_type,
            provider: if self.offline { ProviderChoice::Mock } else { self.provider },
            claude_model: self.claude_model,
            codex_model: self.codex_model,
            output_dir: self.out,
            legal_basis: self.legal_basis,
            retention_days: self.retention_days,
            offline: self.offline,
            verbose: self.verbose,
            ..Default::default()
        };
        if let Some(op) = self.operator {
            cfg.operator = op;
        }
        cfg
    }
}

pub fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Run(mut args) => {
            load_cortex_manifest(&mut args)?;
            let inputs = args.input.clone();
            let cfg = args.into_config();
            run_pipeline(inputs, cfg)
        }
        Command::Menu => menu(),
        Command::Agents { domain } => {
            list_agents(domain);
            Ok(())
        }
        Command::Doctor => doctor(),
        Command::Sources { path } => describe_source(path),
        Command::Init { dir } => init_demo(dir),
        Command::Serve { port, open } => {
            banner();
            crate::serve::serve(port, open)
        }
        Command::Desktop { port } => desktop(port),
    }
}

/// `.cortex` project manifest: a portable JSON that presets a run. If `-i` points
/// to a single `.cortex` file, expand it into the real inputs + domain/out.
/// Example:
///   { "name": "Case 114", "domain": "fraud",
///     "inputs": ["exports/tx.csv", "exports/accounts.csv"], "out": "./out" }
fn load_cortex_manifest(args: &mut RunArgs) -> Result<()> {
    use clap::ValueEnum;
    if args.input.len() != 1 { return Ok(()); }
    let p = &args.input[0];
    if p.extension().map(|e| e != "cortex").unwrap_or(true) { return Ok(()); }
    let text = std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
    let m: serde_json::Value = serde_json::from_str(&text).context("parsing .cortex manifest (must be JSON)")?;
    let base = p.parent().unwrap_or_else(|| std::path::Path::new("."));
    let inputs: Vec<PathBuf> = m.get("inputs").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| { let pb = PathBuf::from(s); if pb.is_absolute() { pb } else { base.join(pb) } }).collect())
        .unwrap_or_default();
    if inputs.is_empty() { return Err(anyhow::anyhow!(".cortex manifest has no 'inputs'")); }
    args.input = inputs;
    if let Some(d) = m.get("domain").and_then(|v| v.as_str()) { if let Ok(dom) = Domain::from_str(d, true) { args.domain = dom; } }
    if let Some(o) = m.get("out").and_then(|v| v.as_str()) { args.out = PathBuf::from(o); }
    println!("{} loaded .cortex manifest ({} input(s), domain={})", "·".dimmed(), args.input.len(), args.domain);
    Ok(())
}

/// Launch the native Tauri app if present; otherwise serve + open the browser.
fn desktop(port: u16) -> Result<()> {
    banner();
    let mut candidates: Vec<String> = vec![
        "/Applications/CortexIntel.app".into(),
        "gui/src-tauri/target/release/bundle/macos/CortexIntel.app".into(),
    ];
    if let Ok(home) = std::env::var("CORTEX_HOME") {
        candidates.push(format!("{home}/gui/src-tauri/target/release/bundle/macos/CortexIntel.app"));
    }
    for app in &candidates {
        if std::path::Path::new(app).exists() {
            println!("launching native app: {app}");
            std::process::Command::new("open").arg(app).spawn()?;
            return Ok(());
        }
    }
    println!("{}", "native app bundle not found — starting local server instead.".yellow());
    println!("{}", "(build the native app with: cd gui/src-tauri && cargo tauri build)".dimmed());
    crate::serve::serve(port, true)
}

fn build_router(cfg: &RunConfig) -> LlmRouter {
    if cfg.offline || cfg.provider == ProviderChoice::Mock {
        LlmRouter::offline(cfg.verbose)
    } else {
        LlmRouter::new(
            cfg.provider,
            cfg.claude_model.clone(),
            cfg.codex_model.clone(),
            cfg.verbose,
        )
    }
}

fn run_pipeline(inputs: Vec<PathBuf>, cfg: RunConfig) -> Result<()> {
    banner();
    println!(
        "vertical={} provider={} type={}\n",
        cfg.domain.slug().bright_white(),
        cfg.provider.to_string().bright_white(),
        cfg.data_type.map(|d| d.slug()).unwrap_or("auto")
    );

    let mut srcs: Vec<Box<dyn sources::DataSource>> = Vec::new();
    for path in &inputs {
        if !path.exists() {
            return Err(anyhow!("input not found: {}", path.display()));
        }
        srcs.push(sources::source_for_path(path, cfg.data_type)?);
    }

    let router = build_router(&cfg);
    pipeline::run(srcs, &cfg, &router)?;
    Ok(())
}

fn list_agents(domain: Domain) {
    banner();
    println!("Agents for vertical: {}\n", domain.title().bright_white());
    for card in agents::catalog(domain) {
        let tag = if card.domain_specialized { "specialist".yellow().to_string() } else { "core".dimmed().to_string() };
        println!("  {} {}", "●".cyan(), card.name.bold());
        println!("    id     {}", card.id.dimmed());
        println!("    stage  {}  [{}]", card.stage.as_str(), tag);
        println!("    role   {}", card.mission);
        println!();
    }
    println!(
        "{}",
        "Each agent runs through the LLM router (Claude → Codex fallback) with a JSON contract.".dimmed()
    );
}

fn doctor() -> Result<()> {
    banner();
    println!("Checking LLM backends…\n");
    let router = LlmRouter::new(ProviderChoice::Auto, None, None, false);
    for (name, res) in router.health_report() {
        match res {
            Ok(v) => println!("  {} {:<8} {}", "✓".green(), name, v.dimmed()),
            Err(e) => println!("  {} {:<8} {}", "✗".red(), name, e.to_string().red()),
        }
    }
    println!();
    println!(
        "{}",
        "Claude runs with `--dangerously-skip-permissions` (subscription); Codex with `codex exec --sandbox read-only`.".dimmed()
    );
    Ok(())
}

fn describe_source(path: PathBuf) -> Result<()> {
    banner();
    if !path.exists() {
        return Err(anyhow!("not found: {}", path.display()));
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if matches!(ext, "mcp" | "yaml" | "yml" | "toml" | "json") && pipeline::read_mcp_manifest(&path).is_ok() {
        if let Ok(plan) = pipeline::read_mcp_manifest(&path) {
            println!("{}", plan);
            return Ok(());
        }
    }
    let src = sources::source_for_path(&path, None)?;
    let batch = src.load()?;
    println!("source   {}", src.describe());
    println!("records  {}", batch.records.len());
    if let Some(first) = batch.records.first() {
        println!("fields   {}", first.fields.keys().cloned().collect::<Vec<_>>().join(", "));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive menu
// ---------------------------------------------------------------------------

fn menu() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
    banner();

    let theme = ColorfulTheme::default();

    // 1) Vertical.
    let domains = Domain::all();
    let domain_labels: Vec<String> = domains.iter().map(|d| d.title().to_string()).collect();
    let d_idx = Select::with_theme(&theme)
        .with_prompt("Business vertical")
        .items(&domain_labels)
        .default(0)
        .interact()?;
    let domain = domains[d_idx];

    // 2) Data type.
    let mut type_labels: Vec<String> = vec!["auto (classify with an agent)".into()];
    type_labels.extend(DataType::all().iter().map(|t| t.slug().to_string()));
    let t_idx = Select::with_theme(&theme)
        .with_prompt("Data type")
        .items(&type_labels)
        .default(0)
        .interact()?;
    let data_type = if t_idx == 0 { None } else { Some(DataType::all()[t_idx - 1]) };

    // 3) Provider.
    let providers = [
        ("Auto (Claude → Codex fallback)", ProviderChoice::Auto),
        ("Claude (subscription)", ProviderChoice::Claude),
        ("ChatGPT Codex", ProviderChoice::Codex),
        ("Offline mock (no LLM)", ProviderChoice::Mock),
    ];
    let p_idx = Select::with_theme(&theme)
        .with_prompt("LLM provider")
        .items(&providers.iter().map(|p| p.0).collect::<Vec<_>>())
        .default(0)
        .interact()?;
    let provider = providers[p_idx].1;

    // 4) Input path(s).
    let input: String = Input::with_theme(&theme)
        .with_prompt("Input source (csv/json/mcp path)")
        .interact_text()?;
    let inputs: Vec<PathBuf> = input.split_whitespace().map(PathBuf::from).collect();

    // 5) Output dir.
    let out: String = Input::with_theme(&theme)
        .with_prompt("Output directory")
        .default("./cortex-out".into())
        .interact_text()?;

    let proceed = Confirm::with_theme(&theme)
        .with_prompt("Run pipeline now?")
        .default(true)
        .interact()?;
    if !proceed {
        println!("aborted.");
        return Ok(());
    }

    let mut cfg = RunConfig {
        domain,
        data_type,
        provider: provider,
        output_dir: PathBuf::from(out),
        offline: provider == ProviderChoice::Mock,
        ..Default::default()
    };
    if cfg.offline {
        cfg.provider = ProviderChoice::Mock;
    }
    run_pipeline(inputs, cfg)
}

// ---------------------------------------------------------------------------
// Demo scaffolding
// ---------------------------------------------------------------------------

fn init_demo(dir: PathBuf) -> Result<()> {
    banner();
    std::fs::create_dir_all(&dir)?;

    let csv = "\
report_id,source_type,case_id,person_id,full_name,account_id,username,ip_address,url_id,full_url,domain,sha256,wallet_address,urgency_level,report_category
R-1001,hotline,C-500,P-1,Alex Doe,A-1,darkfox,203.0.113.9,U-1,https://example.onion/x,example.com,aa11bb22cc33dd44ee55ff6600112233,0x00112233445566778899aabbccddeeff00112233,critical,distribution
R-1002,platform,C-500,P-2,Sam Roe,A-2,nightowl,203.0.113.9,U-2,https://cdn.example.net/f,example.net,bb22cc33dd44ee55ff6600112233aabb,,high,grooming
R-1003,citizen,C-501,P-3,Jamie Kay,A-3,darkfox,198.51.100.7,U-3,https://example.com/p,example.com,,,medium,platform report
";
    let csv_path = dir.join("reports.csv");
    std::fs::write(&csv_path, csv)?;

    let mcp = serde_json::json!({
        "server": "claude_ai_Google_Drive",
        "tool": "search_files",
        "arguments": {"query": "case intake export"},
        "records_path": "files",
        "description": "Pull intake exports from Google Drive via MCP, then re-ingest the rows."
    });
    let mcp_path = dir.join("drive.mcp");
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp)?)?;

    println!("Scaffolded demo in {}", dir.display().bright_white());
    println!("  · {}", csv_path.display());
    println!("  · {}", mcp_path.display());
    println!();
    println!("Try it:");
    println!(
        "  {}",
        format!("cortex run -i {} --domain child-protection --offline", csv_path.display()).cyan()
    );
    println!(
        "  {}",
        format!("cortex run -i {} --domain fraud --provider auto", csv_path.display()).cyan()
    );
    Ok(())
}

fn banner() {
    println!("{}", "CortexIntel".bold().cyan());
}
