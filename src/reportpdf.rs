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

/// Localized report labels (headers, table columns, footer) keyed by language.
struct Rpt { report: &'static str, vertical: &'static str, operator: &'static str, confidential: &'static str, exec: &'static str, case_risk: &'static str, findings: &'static str, leads: &'static str, prioritized: &'static str, col_risk: &'static str, col_kind: &'static str, col_entity: &'static str, col_action: &'static str, actions: &'static str, needs_auth: &'static str, governance: &'static str, retention: &'static str, disposal_on: &'static str, legal_basis: &'static str, disclaimer: &'static str }
fn rpt_labels(lang: &str) -> Rpt {
    match lang {
        "pt" => Rpt { report:"Relatório de Inteligência", vertical:"vertical", operator:"operador", confidential:"Gerado {G} · CONFIDENCIAL — apenas para destinatários autorizados", exec:"Avaliação executiva", case_risk:"Risco do caso", findings:"Principais achados", leads:"Pistas mais fortes", prioritized:"Entidades priorizadas", col_risk:"Risco", col_kind:"Tipo", col_entity:"Entidade", col_action:"Ação", actions:"Ações recomendadas", needs_auth:"(requer autorização)", governance:"Governança e retenção", retention:"Retenção", disposal_on:"descarte em", legal_basis:"Base legal", disclaimer:"A IA apoia a decisão humana. Não decide culpa nem executa ação irreversível. Itens sinalizados exigem revisão humana. Material sensível é referenciado por hash, não reproduzido." },
        "es" => Rpt { report:"Informe de Inteligencia", vertical:"vertical", operator:"operador", confidential:"Generado {G} · CONFIDENCIAL — solo para destinatarios autorizados", exec:"Evaluación ejecutiva", case_risk:"Riesgo del caso", findings:"Hallazgos clave", leads:"Pistas más fuertes", prioritized:"Entidades priorizadas", col_risk:"Riesgo", col_kind:"Tipo", col_entity:"Entidad", col_action:"Acción", actions:"Acciones recomendadas", needs_auth:"(requiere autorización)", governance:"Gobernanza y retención", retention:"Retención", disposal_on:"eliminación el", legal_basis:"Base legal", disclaimer:"La IA apoya la decisión humana. No decide culpabilidad ni ejecuta acción irreversible. Los ítems marcados requieren revisión humana. El material sensible se referencia por hash, no se reproduce." },
        _ => Rpt { report:"Intelligence Report", vertical:"vertical", operator:"operator", confidential:"Generated {G} · CONFIDENTIAL — for authorized recipients only", exec:"Executive assessment", case_risk:"Case risk", findings:"Key findings", leads:"Strongest leads", prioritized:"Prioritized entities", col_risk:"Risk", col_kind:"Kind", col_entity:"Entity", col_action:"Action", actions:"Recommended actions", needs_auth:"(requires authorization)", governance:"Governance & retention", retention:"Retention", disposal_on:"disposal on", legal_basis:"Legal basis", disclaimer:"The AI supports human decision-making. It does not decide guilt or take irreversible action. Flagged items require human review. Sensitive material is referenced by hash, not reproduced." },
    }
}

/// Build the Typst source for a consolidated analysis document.
pub fn build_typst(c: &Value, project_name: &str, domain: &str, operator: &str, generated: &str) -> String {
    let lang = c.get("lang").and_then(|v| v.as_str()).unwrap_or("en");
    let t = rpt_labels(lang);
    let mut s = String::new();
    s.push_str("#set page(paper: \"a4\", margin: 2cm, numbering: \"1 / 1\")\n");
    s.push_str("#set text(size: 10pt, fill: rgb(\"#1a2230\"))\n");
    s.push_str("#set par(justify: true, leading: 0.6em)\n");
    s.push_str("#show heading: set text(fill: rgb(\"#0e3843\"))\n\n");

    // Header
    s.push_str("#block(fill: rgb(\"#0e3843\"), inset: 12pt, radius: 4pt, width: 100%)[\n");
    s.push_str(&format!("  #text(fill: white, size: 16pt, weight: \"bold\")[CortexIntel — {}]\\ \n", t.report));
    s.push_str(&format!("  #text(fill: rgb(\"#9fd8e4\"), size: 9pt)[{} · {}: {} · {}: {}]\n", esc(project_name), t.vertical, esc(domain), t.operator, esc(operator)));
    s.push_str("]\n\n");
    s.push_str(&format!("#text(size: 8pt, fill: gray)[{}]\n\n", t.confidential.replace("{G}", &esc(generated))));

    // Executive assessment
    let summary = get_str(c, &["investigation", "summary"]);
    if !summary.is_empty() {
        s.push_str(&format!("== {}\n", t.exec));
        s.push_str(&esc(summary));
        s.push_str("\n\n");
    }

    // Case risk
    let band = get_str(c, &["ai_assessments", "case_risk_band"]);
    if !band.is_empty() {
        let score = c.get("ai_assessments").and_then(|a| a.get("case_risk_score")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        s.push_str(&format!("*{}:* {} ({:.2})\n\n", t.case_risk, esc(band), score));
    }

    // Key findings
    push_list(&mut s, t.findings, c.get("investigation").and_then(|i| i.get("key_findings")));
    push_list(&mut s, t.leads, c.get("investigation").and_then(|i| i.get("strongest_leads")));

    // Prioritized entities table
    if let Some(arr) = c.get("ai_assessments").and_then(|a| a.get("assessments")).and_then(|v| v.as_array()) {
        if !arr.is_empty() {
            s.push_str(&format!("== {}\n", t.prioritized));
            s.push_str("#table(columns: (auto, auto, 1fr, auto), inset: 5pt, align: left,\n");
            s.push_str(&format!("  [*{}*],[*{}*],[*{}*],[*{}*],\n", t.col_risk, t.col_kind, t.col_entity, t.col_action));
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
            s.push_str(&format!("== {}\n", t.actions));
            for st in steps {
                let action = st.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let auth = st.get("requires_authorization").and_then(|v| v.as_bool()).unwrap_or(false);
                let tag = if auth { format!(" #text(fill: rgb(\"#b8860b\"))[{}]", t.needs_auth) } else { String::new() };
                s.push_str(&format!("- {}{}\n", esc(action), tag));
            }
            s.push_str("\n");
        }
    }

    // Governance
    let retention = c.get("governance").and_then(|g| g.get("retention"));
    s.push_str(&format!("== {}\n", t.governance));
    if let Some(r) = retention {
        let days = r.get("retention_days").and_then(|v| v.as_i64()).unwrap_or(0);
        let disposal = r.get("disposal_date").and_then(|v| v.as_str()).unwrap_or("");
        let basis = r.get("legal_basis").and_then(|v| v.as_str()).unwrap_or("");
        s.push_str(&format!("- {}: {} days → {} {}\n- {}: {}\n", t.retention, days, t.disposal_on, esc(&disposal.chars().take(10).collect::<String>()), t.legal_basis, esc(basis)));
    }
    s.push_str("\n#line(length: 100%, stroke: 0.5pt + gray)\n");
    s.push_str(&format!("#text(size: 8pt, fill: gray)[{}]\n", t.disclaimer));

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
