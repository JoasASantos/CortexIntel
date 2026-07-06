//! Intelligence report → PDF via Typst. Builds a Typst document from a project's
//! consolidated analysis and compiles it with the `typst` CLI. This keeps the
//! report a real intelligence product (exec summary, findings, entities,
//! relationships, actions incl. authorization flags, governance).

use crate::store;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::process::Command;

/// Escape text for inclusion in Typst content mode.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' | '#' | '$' | '*' | '_' | '`' | '@' | '<' | '>' | '[' | ']' | '~' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn get_str<'a>(v: &'a Value, path: &[&str]) -> &'a str {
    let mut cur = v;
    for p in path {
        cur = match cur.get(p) {
            Some(x) => x,
            None => return "",
        };
    }
    cur.as_str().unwrap_or("")
}

/// Build the Typst source for a consolidated analysis document.
pub fn build_typst(c: &Value, project_name: &str, domain: &str, operator: &str, generated: &str) -> String {
    let mut s = String::new();
    s.push_str("#set page(paper: \"a4\", margin: 2cm, numbering: \"1 / 1\")\n");
    s.push_str("#set text(size: 10pt, fill: rgb(\"#1a2230\"))\n");
    s.push_str("#set par(justify: true, leading: 0.6em)\n");
    s.push_str("#show heading: set text(fill: rgb(\"#0e3843\"))\n\n");

    // Header
    s.push_str("#block(fill: rgb(\"#0e3843\"), inset: 12pt, radius: 4pt, width: 100%)[\n");
    s.push_str("  #text(fill: white, size: 16pt, weight: \"bold\")[CortexIntel — Intelligence Report]\\ \n");
    s.push_str(&format!("  #text(fill: rgb(\"#9fd8e4\"), size: 9pt)[{} · vertical: {} · operator: {}]\n", esc(project_name), esc(domain), esc(operator)));
    s.push_str("]\n\n");
    s.push_str(&format!("#text(size: 8pt, fill: gray)[Generated {} · CONFIDENTIAL — for authorized recipients only]\n\n", esc(generated)));

    // Executive assessment
    let summary = get_str(c, &["investigation", "summary"]);
    if !summary.is_empty() {
        s.push_str("== Executive assessment\n");
        s.push_str(&esc(summary));
        s.push_str("\n\n");
    }

    // Case risk
    let band = get_str(c, &["ai_assessments", "case_risk_band"]);
    if !band.is_empty() {
        let score = c.get("ai_assessments").and_then(|a| a.get("case_risk_score")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        s.push_str(&format!("*Case risk:* {} ({:.2})\n\n", esc(band), score));
    }

    // Key findings
    push_list(&mut s, "Key findings", c.get("investigation").and_then(|i| i.get("key_findings")));
    push_list(&mut s, "Strongest leads", c.get("investigation").and_then(|i| i.get("strongest_leads")));

    // Prioritized entities table
    if let Some(arr) = c.get("ai_assessments").and_then(|a| a.get("assessments")).and_then(|v| v.as_array()) {
        if !arr.is_empty() {
            s.push_str("== Prioritized entities\n");
            s.push_str("#table(columns: (auto, auto, 1fr, auto), inset: 5pt, align: left,\n");
            s.push_str("  [*Risk*],[*Kind*],[*Entity*],[*Action*],\n");
            for a in arr.iter().take(20) {
                let r = a.get("risk_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let kind = a.get("entity_kind").and_then(|v| v.as_str()).unwrap_or("");
                let label = a.get("entity_label").and_then(|v| v.as_str()).unwrap_or("");
                let action = a.get("recommended_action").and_then(|v| v.as_str()).unwrap_or("");
                s.push_str(&format!("  [{:.2}],[{}],[{}],[{}],\n", r, esc(kind), esc(label), esc(action)));
            }
            s.push_str(")\n\n");
        }
    }

    // Next steps with authorization flags
    if let Some(steps) = c.get("investigation").and_then(|i| i.get("next_steps")).and_then(|v| v.as_array()) {
        if !steps.is_empty() {
            s.push_str("== Recommended actions\n");
            for st in steps {
                let action = st.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let auth = st.get("requires_authorization").and_then(|v| v.as_bool()).unwrap_or(false);
                let tag = if auth { " #text(fill: rgb(\"#b8860b\"))[(requires authorization)]" } else { "" };
                s.push_str(&format!("- {}{}\n", esc(action), tag));
            }
            s.push_str("\n");
        }
    }

    // Governance
    let retention = c.get("governance").and_then(|g| g.get("retention"));
    s.push_str("== Governance & retention\n");
    if let Some(r) = retention {
        let days = r.get("retention_days").and_then(|v| v.as_i64()).unwrap_or(0);
        let disposal = r.get("disposal_date").and_then(|v| v.as_str()).unwrap_or("");
        let basis = r.get("legal_basis").and_then(|v| v.as_str()).unwrap_or("");
        s.push_str(&format!("- Retention: {} days → disposal on {}\n- Legal basis: {}\n", days, esc(&disposal.chars().take(10).collect::<String>()), esc(basis)));
    }
    s.push_str("\n#line(length: 100%, stroke: 0.5pt + gray)\n");
    s.push_str("#text(size: 8pt, fill: gray)[The AI supports human decision-making. It does not decide guilt or take irreversible action. Flagged items require human review. Sensitive material is referenced by hash, not reproduced.]\n");

    s
}

fn push_list(s: &mut String, title: &str, arr: Option<&Value>) {
    if let Some(a) = arr.and_then(|v| v.as_array()) {
        if !a.is_empty() {
            s.push_str(&format!("== {}\n", title));
            for it in a {
                if let Some(t) = it.as_str() {
                    s.push_str(&format!("- {}\n", esc(t)));
                }
            }
            s.push('\n');
        }
    }
}

/// Compile the consolidated analysis into a PDF; returns the PDF path.
pub fn to_pdf(c: &Value, project_name: &str, domain: &str, operator: &str, generated: &str) -> Result<String> {
    if Command::new("typst").arg("--version").output().map(|o| !o.status.success()).unwrap_or(true) {
        return Err(anyhow!("`typst` CLI not found on PATH — install Typst to export PDFs"));
    }
    let dir = store::uploads_dir();
    store::ensure_dir(&dir)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let typ_path = dir.join(format!("report-{id}.typ"));
    let pdf_path = dir.join(format!("report-{id}.pdf"));
    std::fs::write(&typ_path, build_typst(c, project_name, domain, operator, generated))?;
    let out = Command::new("typst")
        .arg("compile")
        .arg(&typ_path)
        .arg(&pdf_path)
        .output()
        .context("running typst compile")?;
    let _ = std::fs::remove_file(&typ_path);
    if !out.status.success() {
        return Err(anyhow!("typst: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(pdf_path.to_string_lossy().to_string())
}
