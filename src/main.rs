//! CortexIntel — an agnostic data-collection & intelligence engine.
//!
//! The LLM decision layer is provided by the operator's authenticated CLIs
//! (Claude Code with `--dangerously-skip-permissions`, and ChatGPT Codex).
//! Specialized agents run each stage of an ingestion → graph-correlation →
//! risk → investigation → audit pipeline that is domain-agnostic.
//!
//! This binary is a thin wrapper over the `cortexintel` library, which is also
//! driven by the Tauri desktop GUI in `gui/`.

fn main() {
    if let Err(e) = cortexintel::cli::main() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
