//! Consolidated run output. Serializes the graph, assessments, briefs and audit
//! into the "estrutura final do dado consolidado" from DATA.md plus a
//! human-readable Markdown summary.

use crate::audit::{AuditLog, RetentionPolicy};
use crate::config::RunConfig;
use crate::ontology::KnowledgeGraph;
use crate::risk::RiskReport;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Everything a run produces.
pub struct RunOutput<'a> {
    pub config: &'a RunConfig,
    pub graph: &'a KnowledgeGraph,
    pub risk: &'a RiskReport,
    pub investigation: &'a Value,
    pub audit_summary: &'a Value,
    pub audit: &'a AuditLog,
    pub retention: &'a RetentionPolicy,
    pub assessment: &'a [crate::assessment::Assessment],
    pub next_actions: &'a [crate::assessment::NextAction],
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

impl<'a> RunOutput<'a> {
    /// Build the consolidated JSON document (DATA.md "estrutura final").
    pub fn consolidated(&self) -> Value {
        let mut persons = vec![];
        let mut victims = vec![];
        let mut suspects = vec![];
        let mut accounts = vec![];
        let mut devices = vec![];
        let mut ips = vec![];
        let mut urls = vec![];
        let mut groups = vec![];
        let mut payments = vec![];
        let mut media = vec![];
        let mut other = vec![];

        use crate::ontology::EntityKind::*;
        for e in self.graph.entities.values() {
            let v = serde_json::to_value(e).unwrap_or(Value::Null);
            match e.kind {
                Person => persons.push(v),
                Victim => victims.push(v),
                Suspect => suspects.push(v),
                Account => accounts.push(v),
                Device => devices.push(v),
                Ip => ips.push(v),
                Url | Domain => urls.push(v),
                Group => groups.push(v),
                Payment | Wallet => payments.push(v),
                Media | Evidence => media.push(v),
                _ => other.push(v),
            }
        }

        json!({
            "run": {
                "domain": self.config.domain.slug(),
                "provider": self.config.provider.to_string(),
                "operator": self.config.operator,
                "started_at": self.started_at,
                "finished_at": self.finished_at,
            },
            "entities": {
                "persons": persons,
                "victims": victims,
                "suspects": suspects,
                "accounts": accounts,
                "devices": devices,
                "ips": ips,
                "urls": urls,
                "groups": groups,
                "payments": payments,
                "media_artifacts": media,
                "other": other,
            },
            "relationships": self.graph.relationships,
            "ai_assessments": self.risk,
            "assessment": self.assessment,
            "next_best_actions": self.next_actions,
            "investigation": self.investigation,
            "audit_events": self.audit.events,
            "governance": {
                "audit_summary": self.audit_summary,
                "retention": self.retention,
            }
        })
    }

    /// Write all artifacts to the output directory. Returns the paths written.
    pub fn write_all(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let mut written = Vec::new();

        let consolidated_path = dir.join("case.json");
        std::fs::write(
            &consolidated_path,
            serde_json::to_string_pretty(&self.consolidated())?,
        )?;
        written.push(consolidated_path);

        // Graph as nodes+edges for the DESIGN.md graph workspace to load.
        let graph_path = dir.join("graph.json");
        let graph_doc = json!({
            "nodes": self.graph.entities.values().collect::<Vec<_>>(),
            "edges": self.graph.relationships,
        });
        std::fs::write(&graph_path, serde_json::to_string_pretty(&graph_doc)?)?;
        written.push(graph_path);

        let md_path = dir.join("report.md");
        std::fs::write(&md_path, self.markdown())?;
        written.push(md_path);

        Ok(written)
    }

    /// Human-readable Markdown brief.
    pub fn markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("# CortexIntel Run Report\n\n"));
        s.push_str(&format!("- **Vertical:** {}\n", self.config.domain.title()));
        s.push_str(&format!("- **Provider:** {}\n", self.config.provider));
        s.push_str(&format!("- **Operator:** {}\n", self.config.operator));
        s.push_str(&format!(
            "- **Window:** {} → {}\n",
            self.started_at.format("%Y-%m-%d %H:%M:%SZ"),
            self.finished_at.format("%Y-%m-%d %H:%M:%SZ")
        ));
        s.push_str(&format!(
            "- **Entities:** {} · **Relationships:** {}\n",
            self.graph.entity_count(),
            self.graph.relationship_count()
        ));
        s.push_str(&format!(
            "- **Case risk:** {} ({:.2})\n\n",
            self.risk.case_risk_band, self.risk.case_risk_score
        ));

        // Progressive disclosure: the natural-language Assessment comes first
        // (for whoever decides), the tables/graph below (for whoever investigates).
        s.push_str(&crate::assessment::to_markdown(self.assessment));
        s.push_str(&crate::assessment::nba_to_markdown(self.next_actions));

        s.push_str("## Top prioritized entities\n\n");
        s.push_str("| Risk | Band | Kind | Entity | Recommended action | Review |\n");
        s.push_str("|------|------|------|--------|--------------------|--------|\n");
        for a in self.risk.assessments.iter().take(15) {
            s.push_str(&format!(
                "| {:.2} | {} | {} | {} | {} | {} |\n",
                a.risk_score,
                a.risk_band,
                a.entity_kind,
                a.entity_label,
                a.recommended_action,
                if a.requires_human_review { "yes" } else { "no" }
            ));
        }
        s.push('\n');

        if let Some(summary) = self.investigation.get("summary").and_then(|v| v.as_str()) {
            s.push_str("## Investigative brief\n\n");
            s.push_str(summary);
            s.push_str("\n\n");
        }
        if let Some(steps) = self.investigation.get("next_steps").and_then(|v| v.as_array()) {
            if !steps.is_empty() {
                s.push_str("### Next steps\n\n");
                for st in steps {
                    let action = st.get("action").and_then(|v| v.as_str()).unwrap_or("");
                    let auth = st
                        .get("requires_authorization")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    s.push_str(&format!(
                        "- {}{}\n",
                        action,
                        if auth { "  _(requires authorization)_" } else { "" }
                    ));
                }
                s.push('\n');
            }
        }

        s.push_str("## Governance\n\n");
        s.push_str(&format!(
            "- Sensitive-data touches logged: {}\n",
            self.audit.sensitive_touch_count()
        ));
        s.push_str(&format!(
            "- Retention: {} days → disposal on {}\n",
            self.retention.retention_days,
            self.retention.disposal_date.format("%Y-%m-%d")
        ));
        s.push_str(&format!("- Legal basis: {}\n", self.retention.legal_basis));
        s.push_str("\n> The AI supports human decision-making. It does not decide guilt or take irreversible action. All flagged items require human review.\n");
        s
    }

    /// Compact terminal summary.
    pub fn print_terminal(&self) {
        println!();
        println!("{}", "── CortexIntel run complete ──".bold().cyan());
        println!(
            "  vertical   {}",
            self.config.domain.title().bright_white()
        );
        println!(
            "  entities   {}   relationships {}",
            self.graph.entity_count().to_string().green(),
            self.graph.relationship_count().to_string().green()
        );
        println!(
            "  case risk  {} ({:.2})",
            band_colored(&self.risk.case_risk_band),
            self.risk.case_risk_score
        );
        println!("  top entities:");
        for a in self.risk.assessments.iter().take(5) {
            println!(
                "    {:>4.2} {:<8} {:<10} {}",
                a.risk_score,
                band_colored(&a.risk_band),
                a.entity_kind,
                a.entity_label
            );
        }
    }
}

fn band_colored(band: &str) -> String {
    match band {
        "critical" => band.red().bold().to_string(),
        "high" => band.bright_red().to_string(),
        "medium" => band.yellow().to_string(),
        _ => band.green().to_string(),
    }
}
