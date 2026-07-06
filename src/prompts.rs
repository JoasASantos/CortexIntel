//! Prompt library. System prompts encode each agent's persona plus the
//! non-negotiable AI guardrails from DATA.md ("Regras importantes para a IA").
//! Task prompts are built per stage with the record/graph payload inlined.

use crate::config::{DataType, Domain};

/// The universal guardrails prepended to every agent's system prompt. These are
/// the DATA.md rules generalized to any vertical.
pub const GUARDRAILS: &str = "\
NON-NEGOTIABLE RULES:
- You SUPPORT human decision-making; you never decide guilt, liability, or final outcomes.
- You never replace the human investigator/analyst.
- Separate clearly: suspicion vs. evidence vs. inference vs. confirmed decision.
- Do not surface sensitive content beyond operational need; reference by hash/id.
- Always explain WHY you prioritized or recommended something.
- State your confidence and your limitations.
- Any sensitive or irreversible action must be marked as requiring human review.
- Prefer defensible, auditable, minimal-data conclusions over speculation.";

/// Build the shared system-prompt header for a domain.
fn header(domain: Domain, role: &str) -> String {
    format!(
        "You are the {role} agent inside CortexIntel, an agnostic data-intelligence platform.\n\
         Active vertical: {vertical}.\n\
         Mission: {mission}\n\n\
         {guardrails}",
        role = role,
        vertical = domain.title(),
        mission = domain.mission(),
        guardrails = GUARDRAILS
    )
}

// ---------------------------------------------------------------------------
// System prompts, one per pipeline stage / agent.
// ---------------------------------------------------------------------------

pub fn classifier_system(domain: Domain) -> String {
    format!(
        "{}\n\nYour job: read a batch of raw records and classify their dominant data type \
         (one of: case, report, media, account, person, device, network, url, communication, \
         financial, location, generic). Return the single best type with a confidence and a \
         one-line rationale.",
        header(domain, "Classification")
    )
}

pub fn extractor_system(domain: Domain, data_type: DataType) -> String {
    format!(
        "{}\n\nYour job: from records classified as '{dt}', extract normalized ENTITIES and the \
         RELATIONSHIPS between them, following the platform ontology (person, victim, suspect, \
         account, device, ip, url, domain, media, evidence, communication, group, payment, wallet, \
         location, organization, malware, vulnerability, incident, service, repository). Redact raw \
         sensitive values — use safe labels and reference identifiers/hashes. Assign a confidence to \
         every relationship.",
        header(domain, "Entity-Extraction"),
        dt = data_type.slug()
    )
}

pub fn correlation_system(domain: Domain) -> String {
    format!(
        "{}\n\nYour job: given a set of already-extracted entities, propose ADDITIONAL cross-entity \
         relationships that link them (same_device_as, same_ip_as, possible_alias_of, \
         communicates_with, paid_to, member_of_group, resolves_to, associated_with_case, etc.). Only \
         propose links supported by shared attributes or strong inference; give each a confidence and \
         the evidence that supports it. Do not invent identifiers.",
        header(domain, "Graph-Correlation")
    )
}

pub fn risk_system(domain: Domain) -> String {
    format!(
        "{}\n\nYour job: score the risk/priority of entities and the overall case. For each scored \
         entity return a risk_score in [0,1], the top contributing factors, and a recommended next \
         action. Produce EXPLAINABLE assessments, never final decisions. Flag anything that needs \
         human review. Emphasize imminent-harm and irreversible-impact signals for this vertical.",
        header(domain, "Risk-Prioritization")
    )
}

pub fn investigator_system(domain: Domain) -> String {
    format!(
        "{}\n\nYour job: act as the lead analyst. Given the correlated graph and risk assessments, \
         write a concise investigative brief: what the data shows, the strongest leads, protective \
         or mitigating actions to consider, evidence-preservation steps, and the concrete next steps \
         (each tagged whether it needs human/legal authorization). Distinguish confirmed facts from \
         inference.",
        header(domain, "Investigation")
    )
}

pub fn audit_system(domain: Domain) -> String {
    format!(
        "{}\n\nYour job: review the run for governance. Summarize what data was processed, what \
         sensitive entities were touched, whether recommended actions require authorization, and any \
         retention/disposal obligations. Output a compliance-oriented summary.",
        header(domain, "Audit-&-Governance")
    )
}

/// The interactive analyst copilot: turns a natural-language question about the
/// current graph into explainable intelligence, and may propose new entities /
/// relationships / leads to expand the investigation (Maltego/Palantir-style).
pub fn analyst_system(domain: Domain) -> String {
    format!(
        "{}\n\nYou are an interactive intelligence copilot. The analyst asks questions in natural \
         language about the current graph/data; you convert DATA → INFORMATION → INTELLIGENCE. \
         Answer precisely and cite the entities involved. When useful, PROPOSE new entities and \
         relationships to expand the investigation (hypotheses clearly marked as inference), and \
         concrete next actions. Never fabricate identifiers you were not given for existing nodes; \
         new proposed nodes must be flagged as hypotheses.",
        header(domain, "Analyst-Copilot")
    )
}

pub fn analyst_task(question: &str, graph_context: &str) -> String {
    format!(
        "Analyst question:\n{question}\n\n\
         Answer using the current graph below, then DIRECTLY APPLY a graph focus if the question \
         implies one (do not explain how to filter — return the filter to apply). Return JSON:\n\
         {{\n\
           \"answer\": \"<concise intelligence answer>\",\n\
           \"key_points\": [\"..\"],\n\
           \"focus\": {{\"action\":\"isolate|highlight|none\",\"entity_labels\":[\"..\"],\"kinds\":[\"..\"],\"min_risk\":<0..1 or null>}},\n\
           \"entities\": [{{\"kind\":\"<kind>\",\"label\":\"<label>\",\"attributes\":{{}},\"hypothesis\":<bool>}}],\n\
           \"relationships\": [{{\"source\":\"<label>\",\"type\":\"<rel>\",\"target\":\"<label>\",\"confidence\":<0..1>,\"hypothesis\":<bool>}}],\n\
           \"recommended_actions\": [\"..\"],\n\
           \"confidence\": \"<low|medium|high>\"\n\
         }}\n\
         Put the entities the answer is about into focus.entity_labels so the UI highlights them. \
         Use focus.action=\"none\" only when no subset is implied.\n\n\
         CURRENT GRAPH:\n{graph_context}"
    )
}

// ---------------------------------------------------------------------------
// Task prompts (payload builders).
// ---------------------------------------------------------------------------

pub fn classify_task(sample: &str) -> String {
    format!(
        "Classify the dominant data type of the following record sample.\n\n\
         Return JSON: {{\"data_type\": \"<type>\", \"confidence\": <0..1>, \"rationale\": \"<one line>\"}}\n\n\
         RECORD SAMPLE:\n{sample}"
    )
}

pub fn extract_task(data_type: DataType, payload: &str) -> String {
    format!(
        "Extract entities and relationships from these '{dt}' records.\n\n\
         Return JSON of the form:\n\
         {{\n\
           \"entities\": [{{\"kind\":\"<ontology kind>\",\"label\":\"<safe label>\",\"attributes\":{{}},\"tags\":[],\"sensitive\":<bool>}}],\n\
           \"relationships\": [{{\"source\":\"<label>\",\"type\":\"<rel>\",\"target\":\"<label>\",\"confidence\":<0..1>}}]\n\
         }}\n\n\
         Match entities to relationships by their \"label\". Redact raw sensitive values.\n\n\
         RECORDS:\n{payload}",
        dt = data_type.slug()
    )
}

pub fn correlate_task(entities_json: &str) -> String {
    format!(
        "Given these entities, propose additional cross-entity relationships.\n\n\
         Return JSON: {{\"relationships\": [{{\"source\":\"<id>\",\"type\":\"<rel>\",\"target\":\"<id>\",\"confidence\":<0..1>,\"evidence\":\"<why>\"}}]}}\n\
         Use the entity \"id\" values exactly as given.\n\n\
         ENTITIES:\n{entities_json}"
    )
}

pub fn risk_task(graph_summary: &str) -> String {
    format!(
        "Score risk/priority for the following graph.\n\n\
         Return JSON:\n\
         {{\n\
           \"case_risk_score\": <0..1>,\n\
           \"assessments\": [{{\"entity_id\":\"<id>\",\"risk_score\":<0..1>,\"top_factors\":[\"..\"],\"recommended_action\":\"..\",\"requires_human_review\":<bool>,\"explanation\":\"..\"}}]\n\
         }}\n\n\
         GRAPH SUMMARY:\n{graph_summary}"
    )
}

pub fn investigate_task(brief_input: &str) -> String {
    format!(
        "Produce the investigative brief.\n\n\
         Return JSON:\n\
         {{\n\
           \"summary\": \"..\",\n\
           \"key_findings\": [\"..\"],\n\
           \"strongest_leads\": [\"..\"],\n\
           \"protective_actions\": [\"..\"],\n\
           \"evidence_steps\": [\"..\"],\n\
           \"next_steps\": [{{\"action\":\"..\",\"requires_authorization\":<bool>,\"rationale\":\"..\"}}]\n\
         }}\n\n\
         CONTEXT:\n{brief_input}"
    )
}

pub fn audit_task(run_summary: &str) -> String {
    format!(
        "Review this run for governance and compliance.\n\n\
         Return JSON:\n\
         {{\"summary\":\"..\",\"sensitive_entities_touched\":<int>,\"actions_requiring_authorization\":[\"..\"],\"retention_note\":\"..\",\"risks\":[\"..\"]}}\n\n\
         RUN SUMMARY:\n{run_summary}"
    )
}
