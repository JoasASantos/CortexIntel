//! The information → intelligence step: turn the graph + risk into natural-language
//! ASSESSMENTS of the form "{observation} because {evidence}; confidence {x};
//! action: {next step}". Fully deterministic (offline), each statement links back
//! to the entities/relationships that support it. The vertical LENS only changes
//! vocabulary and emphasis — same engine, sharper output per domain.

use crate::config::Domain;
use crate::ontology::{EntityKind, KnowledgeGraph};
use crate::risk::RiskReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// Natural-language statement (already lens-flavored for the vertical).
    pub statement: String,
    /// Calibrated confidence [0..1].
    pub confidence: f32,
    /// Human evidence references (entity labels / relationship phrasings).
    pub evidence: Vec<String>,
    /// Entity ids the statement is anchored to (for the GUI to focus).
    pub evidence_ids: Vec<String>,
    /// Recommended next step.
    pub action: String,
    /// "observed" (from data) vs "inferred" (structural inference).
    pub basis: String,
    /// Who/what produced this statement (attribution for the decision panel).
    #[serde(default = "deterministic_engine")]
    pub attributed_to: String,
}

/// Per-vertical vocabulary lens. Same structure, different words/emphasis —
/// localized to the run's language so the generated intelligence reads natively.
struct Lens {
    picture: String,
    hub_meaning: String,
    actor: String,
    escalate: String,
}
fn ls(picture: &str, hub: &str, actor: &str, esc: &str) -> Lens {
    Lens { picture: picture.into(), hub_meaning: hub.into(), actor: actor.into(), escalate: esc.into() }
}

fn lens(d: Domain, lang: &str) -> Lens {
    match lang {
        "pt" => match d {
            Domain::Cybersecurity => ls("panorama de ameaças", "infraestrutura compartilhada costuma indicar atividade coordenada ou um operador em comum", "agente de ameaça", "encaminhar ao SOC/DFIR para contenção"),
            Domain::Fraud | Domain::Kyc | Domain::Finance => ls("panorama de fraude", "contas que compartilham dispositivo/IP/carteira frequentemente sinalizam uma rede de laranjas ou um único controlador", "conta de alta exposição", "abrir uma investigação financeira e bloquear até revisão"),
            Domain::ChildProtection => ls("panorama de proteção à vítima", "contas/infraestrutura compartilhadas podem ligar uma rede de distribuição", "entidade em risco ou suspeita", "escalar à proteção infantil e preservar as evidências"),
            Domain::Commerce | Domain::Education => ls("panorama do cliente", "contas agrupadas por um atributo comum podem ser um mesmo domicílio ou um segmento coordenado", "conta prioritária", "encaminhar à equipe responsável para contato ou revisão"),
            Domain::Logistics => ls("panorama operacional", "ativos convergindo em um nó podem ser um gargalo ou ponto único de falha", "ativo crítico", "sinalizar para revisão operacional"),
            Domain::Military => ls("panorama situacional", "entidades convergindo em um nó podem indicar coordenação ou um facilitador-chave", "entidade de interesse", "encaminhar à revisão de um analista humano — nunca ação automatizada"),
            _ => ls("panorama de inteligência", "entidades que compartilham um atributo estão correlacionadas e merecem análise conjunta", "entidade prioritária", "encaminhar para revisão humana"),
        },
        "es" => match d {
            Domain::Cybersecurity => ls("panorama de amenazas", "la infraestructura compartida suele indicar actividad coordinada o un operador común", "actor de amenaza", "derivar al SOC/DFIR para contención"),
            Domain::Fraud | Domain::Kyc | Domain::Finance => ls("panorama de fraude", "las cuentas que comparten dispositivo/IP/billetera suelen señalar una red de mulas o un único controlador", "cuenta de alta exposición", "abrir una investigación financiera y congelar hasta revisión"),
            Domain::ChildProtection => ls("panorama de protección de la víctima", "las cuentas/infraestructura compartidas pueden vincular una red de distribución", "entidad en riesgo o sospechosa", "escalar a protección infantil y preservar la evidencia"),
            Domain::Commerce | Domain::Education => ls("panorama del cliente", "las cuentas agrupadas por un atributo común pueden ser un mismo hogar o un segmento coordinado", "cuenta prioritaria", "derivar al equipo responsable para contacto o revisión"),
            Domain::Logistics => ls("panorama operativo", "los activos que convergen en un nodo pueden ser un cuello de botella o punto único de fallo", "activo crítico", "marcar para revisión operativa"),
            Domain::Military => ls("panorama situacional", "las entidades que convergen en un nodo pueden indicar coordinación o un facilitador clave", "entidad de interés", "derivar a la revisión de un analista humano — nunca acción automatizada"),
            _ => ls("panorama de inteligencia", "las entidades que comparten un atributo están correlacionadas y conviene examinarlas juntas", "entidad prioritaria", "derivar para revisión humana"),
        },
        _ => match d {
            Domain::Cybersecurity => ls("threat picture", "shared infrastructure often indicates coordinated activity or a common operator", "threat actor", "hand to SOC/DFIR for containment"),
            Domain::Fraud | Domain::Kyc | Domain::Finance => ls("fraud picture", "accounts sharing a device/IP/wallet frequently signal a mule ring or single controller", "high-exposure account", "open a financial investigation and freeze pending review"),
            Domain::ChildProtection => ls("victim-protection picture", "shared accounts/infrastructure can link a distribution network", "at-risk or suspect entity", "escalate to child-protection and preserve evidence"),
            Domain::Commerce | Domain::Education => ls("customer picture", "accounts clustering on a shared attribute may be one household or a coordinated segment", "priority account", "route to the owning team for outreach or review"),
            Domain::Logistics => ls("operations picture", "assets converging on a node may be a bottleneck or single point of failure", "critical asset", "flag for operational review"),
            Domain::Military => ls("situational picture", "entities converging on a node may indicate coordination or a key facilitator", "entity of interest", "route to human analyst review — never automated action"),
            _ => ls("intelligence picture", "entities sharing an attribute are correlated and worth examining together", "priority entity", "route for human review"),
        },
    }
}

/// Localized noun for an entity kind, for use inside generated sentences.
fn kind_label(k: EntityKind, lang: &str) -> String {
    use EntityKind::*;
    let en = k.as_str();
    let s = match lang {
        "pt" => match k { Person=>"pessoa",Account=>"conta",Organization=>"organização",Ip=>"IP",Domain=>"domínio",Url=>"URL",Device=>"dispositivo",Wallet=>"carteira",Payment=>"pagamento",Group=>"grupo",Location=>"local",Media=>"mídia",Evidence=>"evidência",Malware=>"malware",Incident=>"incidente",Vulnerability=>"vulnerabilidade",Suspect=>"suspeito",Victim=>"vítima",Communication=>"comunicação",_=>en },
        "es" => match k { Person=>"persona",Account=>"cuenta",Organization=>"organización",Ip=>"IP",Domain=>"dominio",Url=>"URL",Device=>"dispositivo",Wallet=>"billetera",Payment=>"pago",Group=>"grupo",Location=>"ubicación",Media=>"medio",Evidence=>"evidencia",Malware=>"malware",Incident=>"incidente",Vulnerability=>"vulnerabilidad",Suspect=>"sospechoso",Victim=>"víctima",Communication=>"comunicación",_=>en },
        _ => en,
    };
    s.to_string()
}

/// Localized label for a case-risk band.
fn band_label(band: &str, lang: &str) -> &'static str {
    match (lang, band) {
        ("pt","critical")=>"crítico",("pt","high")=>"alto",("pt","medium")=>"médio",("pt",_)=>"baixo",
        ("es","critical")=>"crítico",("es","high")=>"alto",("es","medium")=>"medio",("es",_)=>"bajo",
        (_,"critical")=>"critical",(_,"high")=>"high",(_,"medium")=>"medium",_=>"low",
    }
}

fn degrees(g: &KnowledgeGraph) -> HashMap<String, usize> {
    g.degree_centrality()
}

/// Build the assessment for a run. Deterministic; ordered by confidence.
/// `lang` localizes the generated natural-language text ("en" | "pt" | "es").
pub fn assess(g: &KnowledgeGraph, risk: &RiskReport, domain: Domain, lang: &str) -> Vec<Assessment> {
    let l = lens(domain, lang);
    let deg = degrees(g);
    let mut out: Vec<Assessment> = Vec::new();
    let bl = band_label(&risk.case_risk_band, lang);

    // 1) Overall posture from case risk.
    let band = risk.case_risk_band.as_str();
    let conf = match band { "critical" => 0.8, "high" => 0.7, "medium" => 0.5, _ => 0.4 };
    let (statement, evidence, action) = match lang {
        "pt" => {
            let verb = match band { "critical"=>"exige atenção imediata","high"=>"requer revisão prioritária","medium"=>"apresenta sinais moderados",_=>"aparenta baixo sinal" };
            (format!("O {} {} — o risco geral do caso é {} ({:.2}).", l.picture, verb, bl, risk.case_risk_score),
             vec![format!("{} entidades, {} relações", g.entity_count(), g.relationship_count())],
             format!("Revise as entidades prioritárias; {} se confirmado.", l.escalate))
        }
        "es" => {
            let verb = match band { "critical"=>"exige atención inmediata","high"=>"requiere revisión prioritaria","medium"=>"muestra señales moderadas",_=>"parece de baja señal" };
            (format!("El {} {} — el riesgo general del caso es {} ({:.2}).", l.picture, verb, bl, risk.case_risk_score),
             vec![format!("{} entidades, {} relaciones", g.entity_count(), g.relationship_count())],
             format!("Revisa las entidades prioritarias; {} si se confirma.", l.escalate))
        }
        _ => {
            let verb = match band { "critical"=>"demands immediate attention","high"=>"warrants prioritized review","medium"=>"shows moderate signals",_=>"appears low-signal" };
            (format!("The {} {} — overall case risk is {} ({:.2}).", l.picture, verb, bl, risk.case_risk_score),
             vec![format!("{} entities, {} relationships", g.entity_count(), g.relationship_count())],
             format!("Review the top prioritized entities; {} if confirmed.", l.escalate))
        }
    };
    out.push(Assessment { statement, confidence: conf, evidence, evidence_ids: vec![], action, basis: "observed".into(), attributed_to: deterministic_engine() });

    // 2) Shared-hub coordination (structural inference).
    let hub_kinds = [EntityKind::Ip, EntityKind::Device, EntityKind::Wallet, EntityKind::Domain, EntityKind::Group];
    let mut hubs: Vec<(&String, usize, EntityKind)> = g.entities.iter()
        .filter(|(_, e)| hub_kinds.contains(&e.kind))
        .map(|(id, e)| (id, *deg.get(id).unwrap_or(&0), e.kind))
        .filter(|(_, d, _)| *d >= 3).collect();
    hubs.sort_by(|a, b| b.1.cmp(&a.1));
    if let Some((id, d, kind)) = hubs.first() {
        let e = &g.entities[*id];
        let lk = (0.35 + *d as f32 * 0.05).min(0.9);
        let kl = kind_label(*kind, lang);
        let (statement, evidence, action) = match lang {
            "pt" => (format!("{} entidades convergem no {} compartilhado \"{}\" — {}.", d, kl, e.label, l.hub_meaning),
                vec![format!("{} conexões com {}", d, e.label)],
                "Isole este cluster e expanda seus membros; confirme se o hub compartilhado é um vínculo real ou um agregador benigno.".to_string()),
            "es" => (format!("{} entidades convergen en el {} compartido \"{}\" — {}.", d, kl, e.label, l.hub_meaning),
                vec![format!("{} conexiones con {}", d, e.label)],
                "Aísla este clúster y expande sus miembros; confirma si el hub compartido es un vínculo real o un agregador benigno.".to_string()),
            _ => (format!("{} entities converge on the shared {} \"{}\" — {}.", d, kl, e.label, l.hub_meaning),
                vec![format!("{} connections to {}", d, e.label)],
                "Isolate this cluster and expand its members; confirm whether the shared hub is a genuine link or a benign aggregator.".to_string()),
        };
        out.push(Assessment { statement, confidence: lk, evidence, evidence_ids: vec![(*id).clone()], action, basis: "inferred".into(), attributed_to: deterministic_engine() });
    }

    // 3) Concentration of risk in few actors.
    let mut ranked: Vec<_> = risk.assessments.iter().filter(|a| a.risk_score >= 0.6).collect();
    ranked.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());
    if !ranked.is_empty() {
        let top = &ranked[..ranked.len().min(3)];
        let names: Vec<String> = top.iter().map(|a| a.entity_label.clone()).collect();
        let plural = top.len() > 1;
        let (statement, action) = match lang {
            "pt" => (format!("O risco se concentra em {} {}{}: {}.", top.len(), l.actor, if plural {"s"} else {""}, names.join(", ")),
                format!("Verifique primeiro {} {}{}; {}.", if plural {"as"} else {"a"}, l.actor, if plural {"s"} else {""}, l.escalate)),
            "es" => (format!("El riesgo se concentra en {} {}{}: {}.", top.len(), l.actor, if plural {"s"} else {""}, names.join(", ")),
                format!("Verifica primero {} {}{}; {}.", if plural {"las"} else {"la"}, l.actor, if plural {"s"} else {""}, l.escalate)),
            _ => (format!("Risk concentrates in {} {}{}: {}.", top.len(), l.actor, if plural {"s"} else {""}, names.join(", ")),
                format!("Verify the top {}{} first; {}.", l.actor, if plural {"s"} else {""}, l.escalate)),
        };
        out.push(Assessment { statement, confidence: (top[0].risk_score * 0.9).min(0.85),
            evidence: top.iter().map(|a| format!("{} — {} ({:.2})", a.entity_label, a.risk_band, a.risk_score)).collect(),
            evidence_ids: top.iter().map(|a| a.entity_id.clone()).collect(), action, basis: "observed".into(), attributed_to: deterministic_engine() });
    }

    // 4) Duplicate/identity collision (data-quality caveat that bounds confidence).
    let mut by_key: HashMap<String, usize> = HashMap::new();
    for e in g.entities.values() { *by_key.entry(format!("{}|{}", e.kind.as_str(), e.label.to_lowercase())).or_insert(0) += 1; }
    let dups: usize = by_key.values().filter(|c| **c > 1).map(|c| *c).sum();
    if dups > 1 {
        let (statement, evidence, action) = match lang {
            "pt" => (format!("{} entidades parecem ser duplicatas ou identidades confundidas, o que pode distorcer clusters e inflar o risco.", dups),
                vec![format!("{} colisões de rótulo detectadas", dups)],
                "Resolva as duplicatas antes de tirar conclusões firmes — trate os tamanhos de cluster como limites superiores.".to_string()),
            "es" => (format!("{} entidades parecen ser duplicados o identidades confundidas, lo que puede distorsionar clústeres e inflar el riesgo.", dups),
                vec![format!("{} colisiones de etiqueta detectadas", dups)],
                "Resuelve los duplicados antes de sacar conclusiones firmes — trata los tamaños de clúster como límites superiores.".to_string()),
            _ => (format!("{} entities appear to be duplicates or conflated identities, which can distort clusters and inflate risk.", dups),
                vec![format!("{} label collisions detected", dups)],
                "Resolve duplicates before drawing firm conclusions — treat cluster sizes as upper bounds.".to_string()),
        };
        out.push(Assessment { statement, confidence: 0.6, evidence, evidence_ids: vec![], action, basis: "observed".into(), attributed_to: deterministic_engine() });
    }

    // 5) Reference-source (known-hash) matches — an exact, observed hit against an
    // integrated feed (known-CSAM set, malware hashes, watchlist). High confidence
    // because a hash match is exact; the identity of what it IS rests on the source.
    let matches: Vec<&crate::ontology::Entity> = g
        .entities
        .values()
        .filter(|e| e.attributes.contains_key("ref_source"))
        .collect();
    if !matches.is_empty() {
        let exact: Vec<&crate::ontology::Entity> = matches.iter().copied()
            .filter(|e| e.attributes.get("ref_match").map(|m| m != "perceptual").unwrap_or(true)).collect();
        let perceptual: Vec<&crate::ontology::Entity> = matches.iter().copied()
            .filter(|e| e.attributes.get("ref_match").map(|m| m == "perceptual").unwrap_or(false)).collect();

        // 5a) Exact file-hash matches — definitive, already-catalogued material.
        if !exact.is_empty() {
            let srcs: Vec<String> = { let mut s: Vec<String> = exact.iter().filter_map(|e| e.attributes.get("ref_source").cloned()).collect(); s.sort(); s.dedup(); s };
            let cat = exact[0].attributes.get("ref_category").cloned().unwrap_or_default();
            let statement = match lang {
                "pt" => format!("{} arquivo(s) batem EXATAMENTE com base(s) de material conhecido ({}), categoria \"{}\" — material já catalogado.", exact.len(), srcs.join(", "), cat),
                "es" => format!("{} archivo(s) coinciden EXACTAMENTE con base(s) de material conocido ({}), categoría \"{}\" — material ya catalogado.", exact.len(), srcs.join(", "), cat),
                _ => format!("{} file(s) EXACTLY match known-material reference source(s) ({}), category \"{}\" — already-catalogued material.", exact.len(), srcs.join(", "), cat),
            };
            let action = match lang { "pt" => format!("Confirmação humana obrigatória; {}.", l.escalate), "es" => format!("Confirmación humana obligatoria; {}.", l.escalate), _ => format!("Mandatory human confirmation; {}.", l.escalate) };
            out.push(Assessment { statement, confidence: 0.95, evidence: exact.iter().take(5).map(|e| e.label.clone()).collect(), evidence_ids: exact.iter().map(|e| e.id.clone()).collect(), action, basis: "observed".into(), attributed_to: deterministic_engine() });
        }

        // 5b) Perceptual near-duplicate matches — likely altered/recompressed copy
        // of known material. Confidence scaled by the (worst) similarity seen.
        if !perceptual.is_empty() {
            let srcs: Vec<String> = { let mut s: Vec<String> = perceptual.iter().filter_map(|e| e.attributes.get("ref_source").cloned()).collect(); s.sort(); s.dedup(); s };
            let sims: Vec<f32> = perceptual.iter().filter_map(|e| e.attributes.get("ref_similarity").and_then(|v| v.parse().ok())).collect();
            let min_sim = sims.iter().cloned().fold(1.0f32, f32::min);
            let pct = (min_sim * 100.0).round() as i32;
            let statement = match lang {
                "pt" => format!("{} arquivo(s) são quase-duplicatas de material conhecido ({}) — similaridade ≥{}%, provável cópia recomprimida/alterada.", perceptual.len(), srcs.join(", "), pct),
                "es" => format!("{} archivo(s) son casi-duplicados de material conocido ({}) — similitud ≥{}%, probable copia recomprimida/alterada.", perceptual.len(), srcs.join(", "), pct),
                _ => format!("{} file(s) are near-duplicates of known material ({}) — ≥{}% similar, likely a recompressed/altered copy.", perceptual.len(), srcs.join(", "), pct),
            };
            let action = match lang { "pt" => format!("Confirmação humana obrigatória (match por similaridade, não exato); {}.", l.escalate), "es" => format!("Confirmación humana obligatoria (coincidencia por similitud, no exacta); {}.", l.escalate), _ => format!("Mandatory human confirmation (similarity match, not exact); {}.", l.escalate) };
            out.push(Assessment { statement, confidence: (min_sim * 0.9).clamp(0.5, 0.9), evidence: perceptual.iter().take(5).map(|e| format!("{} (~{:.0}%)", e.label, e.attributes.get("ref_similarity").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) * 100.0)).collect(), evidence_ids: perceptual.iter().map(|e| e.id.clone()).collect(), action, basis: "observed".into(), attributed_to: deterministic_engine() });
        }
    }

    // 6) Network structure — the broker (top betweenness) and emergent communities.
    // Reads the metrics network science wrote onto the entities.
    let mut broker: Option<(&crate::ontology::Entity, f32)> = None;
    let mut comms: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in g.entities.values() {
        if let Some(c) = e.attributes.get("community") { comms.insert(c.clone()); }
        if let Some(b) = e.attributes.get("betweenness").and_then(|v| v.parse::<f32>().ok()) {
            if broker.map(|(_, bb)| b > bb).unwrap_or(true) { broker = Some((e, b)); }
        }
    }
    if let Some((e, b)) = broker {
        if b >= 0.15 {
            let kl = kind_label(e.kind, lang);
            let (statement, action) = match lang {
                "pt" => (
                    format!("O {} \"{}\" é o principal ponto de articulação da rede (intermediação {:.2}) — conecta partes que, sem ele, ficariam separadas; provável facilitador/elo central.", kl, e.label, b),
                    "Priorize este nó: expanda-o, confirme o vínculo e avalie o impacto de removê-lo (fragmentação da rede).".to_string(),
                ),
                "es" => (
                    format!("El {} \"{}\" es el principal punto de articulación de la red (intermediación {:.2}) — conecta partes que sin él quedarían separadas; probable facilitador/enlace central.", kl, e.label, b),
                    "Prioriza este nodo: expándelo, confirma el vínculo y evalúa el impacto de removerlo (fragmentación de la red).".to_string(),
                ),
                _ => (
                    format!("The {} \"{}\" is the network's main broker (betweenness {:.2}) — it connects parts that would otherwise be separate; likely a facilitator / central link.", kl, e.label, b),
                    "Prioritize this node: expand it, confirm the link, and assess the impact of removing it (network fragmentation).".to_string(),
                ),
            };
            out.push(Assessment { statement, confidence: (0.4 + b * 0.5).min(0.85), evidence: vec![format!("betweenness {:.2}", b)], evidence_ids: vec![e.id.clone()], action, basis: "inferred".into(), attributed_to: deterministic_engine() });
        }
    }
    if comms.len() >= 2 {
        let (statement, action) = match lang {
            "pt" => (format!("A rede se organiza em {} comunidades distintas — subgrupos internamente mais conectados que entre si.", comms.len()),
                "Analise cada comunidade como uma unidade; ligações ENTRE comunidades costumam ser as pontes mais informativas.".to_string()),
            "es" => (format!("La red se organiza en {} comunidades distintas — subgrupos más conectados internamente que entre sí.", comms.len()),
                "Analiza cada comunidad como una unidad; los enlaces ENTRE comunidades suelen ser los puentes más informativos.".to_string()),
            _ => (format!("The network organizes into {} distinct communities — subgroups more connected within than between.", comms.len()),
                "Examine each community as a unit; links BETWEEN communities are usually the most informative bridges.".to_string()),
        };
        out.push(Assessment { statement, confidence: 0.55, evidence: vec![format!("{} communities", comms.len())], evidence_ids: vec![], action, basis: "inferred".into(), attributed_to: deterministic_engine() });
    }

    // 7) Anomalies — entities that stand out from their same-kind peers.
    let mut anoms: Vec<(&crate::ontology::Entity, f32, String)> = g.entities.values()
        .filter_map(|e| {
            let s = e.attributes.get("anomaly_score").and_then(|v| v.parse::<f32>().ok())?;
            let r = e.attributes.get("anomaly_reason").cloned().unwrap_or_default();
            Some((e, s, r))
        }).collect();
    if !anoms.is_empty() {
        anoms.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let (top, tscore, treason) = (&anoms[0].0, anoms[0].1, anoms[0].2.clone());
        let kl = kind_label(top.kind, lang);
        let (statement, action) = match lang {
            "pt" => (
                format!("{} entidade(s) destoam dos seus pares. A mais destacada: o {} \"{}\" — {}.", anoms.len(), kl, top.label, treason),
                "Revise os outliers: um valor fora da curva costuma ser conta-ponte, coletor ou erro de dado — vale confirmar qual.".to_string(),
            ),
            "es" => (
                format!("{} entidad(es) se desvían de sus pares. La más destacada: el {} \"{}\" — {}.", anoms.len(), kl, top.label, treason),
                "Revisa los outliers: un valor fuera de rango suele ser cuenta-puente, recolector o error de dato — conviene confirmar cuál.".to_string(),
            ),
            _ => (
                format!("{} entit(y/ies) deviate from their peers. Most notable: the {} \"{}\" — {}.", anoms.len(), kl, top.label, treason),
                "Review the outliers: an off-the-curve value is often a bridge account, a collector, or a data error — worth confirming which.".to_string(),
            ),
        };
        out.push(Assessment { statement, confidence: (0.45 + tscore * 0.4).min(0.85), evidence: anoms.iter().take(4).map(|(e, _, r)| format!("{} — {}", e.label, r)).collect(), evidence_ids: anoms.iter().take(6).map(|(e, _, _)| e.id.clone()).collect(), action, basis: "inferred".into(), attributed_to: deterministic_engine() });
    }

    // 8) Predicted links — likely-but-absent edges inferred from shared structure.
    let preds: Vec<&crate::ontology::Relationship> = g.relationships.iter().filter(|r| r.rel_type == "predicted_link").collect();
    if !preds.is_empty() {
        let mut top = preds.clone();
        top.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let strongest = top[0];
        let la = g.entities.get(&strongest.source_id).map(|e| e.label.clone()).unwrap_or_default();
        let lb = g.entities.get(&strongest.target_id).map(|e| e.label.clone()).unwrap_or_default();
        let shared = strongest.attributes.get("shared_neighbors").cloned().unwrap_or_default();
        let (statement, action) = match lang {
            "pt" => (
                format!("{} vínculo(s) provável(is) mas AUSENTE(S) foram inferidos por estrutura compartilhada. O mais forte: \"{}\" ↔ \"{}\" ({} vizinhos em comum) — provavelmente conectados, sem aresta direta no dado.", preds.len(), la, lb, shared),
                "Verifique esses pares: podem ser uma aresta que faltou coletar ou um vínculo oculto a confirmar.".to_string(),
            ),
            "es" => (
                format!("{} vínculo(s) probable(s) pero AUSENTE(S) fueron inferidos por estructura compartida. El más fuerte: \"{}\" ↔ \"{}\" ({} vecinos en común) — probablemente conectados, sin arista directa en el dato.", preds.len(), la, lb, shared),
                "Verifica esos pares: pueden ser una arista no recolectada o un vínculo oculto a confirmar.".to_string(),
            ),
            _ => (
                format!("{} likely-but-ABSENT link(s) inferred from shared structure. Strongest: \"{}\" ↔ \"{}\" ({} shared neighbours) — probably connected, with no direct edge in the data.", preds.len(), la, lb, shared),
                "Verify these pairs: they may be an edge you didn't collect, or a hidden relationship to confirm.".to_string(),
            ),
        };
        out.push(Assessment { statement, confidence: (strongest.confidence * 0.8).clamp(0.3, 0.7), evidence: top.iter().take(4).map(|r| format!("{} ↔ {}", g.entities.get(&r.source_id).map(|e| e.label.as_str()).unwrap_or("?"), g.entities.get(&r.target_id).map(|e| e.label.as_str()).unwrap_or("?"))).collect(), evidence_ids: vec![strongest.source_id.clone(), strongest.target_id.clone()], action, basis: "inferred".into(), attributed_to: deterministic_engine() });
    }

    out.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    out
}

/// A ranked next-best-action: the collection/verification that most reduces the
/// investigation's uncertainty, with the reason and estimated payoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub action: String,
    pub why: String,
    /// Estimated uncertainty reduction if done [0..1].
    pub uncertainty_reduction: f32,
    /// Estimated effort [0..1] (higher = costlier).
    pub effort: f32,
    /// Composite priority = value / effort, normalized [0..1].
    pub priority: f32,
    /// Where to do it (a GUI destination hint).
    pub target: String,
    /// Entity ids this action concerns (for GUI focus), if any.
    pub entity_ids: Vec<String>,
    /// Who/what produced this action (attribution for the decision panel).
    #[serde(default = "deterministic_engine")]
    pub attributed_to: String,
    /// Rough effort→time hint in hours, for the planning timeline (G4).
    #[serde(default)]
    pub est_hours: f32,
}

fn deterministic_engine() -> String { "Deterministic engine".to_string() }

/// Rank the next-best-actions by how much they cut uncertainty per unit effort.
/// Deterministic: derived from real data-quality/structure gaps, not opinion.
pub fn next_best_actions(g: &KnowledgeGraph, risk: &RiskReport, domain: Domain, lang: &str) -> Vec<NextAction> {
    let l = lens(domain, lang);
    let deg = degrees(g);
    let n = g.entity_count().max(1);
    let mut acts: Vec<NextAction> = Vec::new();

    let mut mk = |action: String, why: String, red: f32, effort: f32, target: &str, ids: Vec<String>| {
        let effort = effort.max(0.05);
        let est_hours = (0.5 + effort * 7.5).round();
        acts.push(NextAction { action, why, uncertainty_reduction: red, effort, priority: (red / effort).min(3.0) / 3.0, target: target.into(), entity_ids: ids, attributed_to: deterministic_engine(), est_hours });
    };
    let pt = lang == "pt"; let es = lang == "es";

    // 1) Entities with no source — provenance gaps cap trust.
    let no_src: Vec<&crate::ontology::Entity> = g.entities.values().filter(|e| e.sources.is_empty()).collect();
    if !no_src.is_empty() {
        let frac = no_src.len() as f32 / n as f32; let c = no_src.len();
        let (a, w) = if pt { (format!("Rastrear a origem de {} entidades sem proveniência", c), "Entidades sem fonte não são confiáveis; estabelecer a proveniência eleva diretamente a confiança em todo julgamento que depende delas.".to_string()) }
            else if es { (format!("Rastrear el origen de {} entidades sin procedencia", c), "Las entidades sin fuente no son fiables; establecer la procedencia eleva directamente la confianza en cada juicio que depende de ellas.".to_string()) }
            else { (format!("Trace the source of {} entities lacking provenance", c), "Unsourced entities can't be trusted; establishing provenance directly raises confidence in every judgment that depends on them.".to_string()) };
        mk(a, w, (0.35 + frac * 0.4).min(0.8), 0.4, "entities", no_src.iter().take(20).map(|e| e.id.clone()).collect());
    }

    // 2) Ambiguous shared hub — resolving benign-vs-real is high leverage.
    let hub_kinds = [EntityKind::Ip, EntityKind::Device, EntityKind::Wallet, EntityKind::Domain];
    if let Some((id, d)) = g.entities.iter().filter(|(_, e)| hub_kinds.contains(&e.kind))
        .map(|(id, _)| (id, *deg.get(id).unwrap_or(&0))).filter(|(_, d)| *d >= 3).max_by_key(|(_, d)| *d) {
        let e = &g.entities[id]; let kl = kind_label(e.kind, lang);
        let (a, w) = if pt { (format!("Verificar se o {} compartilhado \"{}\" é um vínculo real ou um agregador benigno", kl, e.label), format!("{} entidades dependem deste hub — {}. Confirmá-lo colapsa ou confirma o significado de todo o cluster.", d, l.hub_meaning)) }
            else if es { (format!("Verificar si el {} compartido \"{}\" es un vínculo real o un agregador benigno", kl, e.label), format!("{} entidades dependen de este hub — {}. Confirmarlo colapsa o confirma el significado de todo el clúster.", d, l.hub_meaning)) }
            else { (format!("Verify whether the shared {} \"{}\" is a real link or a benign aggregator", kl, e.label), format!("{} entities hinge on this hub — {}. Confirming it collapses or confirms the whole cluster's meaning.", d, l.hub_meaning)) };
        mk(a, w, 0.6, 0.3, "graph", vec![id.clone()]);
    }

    // 3) Isolated entities — correlation is incomplete.
    let isolated = g.entities.keys().filter(|id| *deg.get(*id).unwrap_or(&0) == 0).count();
    if isolated > 0 {
        let (a, w) = if pt { (format!("Enriquecer {} entidades isoladas para revelar relações", isolated), "Entidades sem relações conhecidas deixam o panorama subconectado; enriquecê-las pode revelar vínculos que mudam clusters e risco.".to_string()) }
            else if es { (format!("Enriquecer {} entidades aisladas para revelar relaciones", isolated), "Las entidades sin relaciones conocidas dejan el panorama subconectado; enriquecerlas puede revelar vínculos que cambian clústeres y riesgo.".to_string()) }
            else { (format!("Enrich {} isolated entities to reveal relationships", isolated), "Entities with no known relations mean the picture is under-connected; enriching them can surface links that change clusters and risk.".to_string()) };
        mk(a, w, (0.25 + (isolated as f32 / n as f32) * 0.35).min(0.7), 0.55, "sources", vec![]);
    }

    // 4) Unconfirmed AI hypotheses — verify before relying on them.
    let hyp: Vec<&crate::ontology::Entity> = g.entities.values().filter(|e| e.tags.iter().any(|t| t == "hypothesis")).collect();
    if !hyp.is_empty() {
        let (a, w) = if pt { (format!("Confirmar ou rejeitar {} entidades propostas pela IA", hyp.len()), "Hipóteses de IA são inferência, não evidência; validá-las remove a maior fonte de risco especulativo no grafo.".to_string()) }
            else if es { (format!("Confirmar o rechazar {} entidades propuestas por la IA", hyp.len()), "Las hipótesis de IA son inferencia, no evidencia; validarlas elimina la mayor fuente de riesgo especulativo en el grafo.".to_string()) }
            else { (format!("Confirm or reject {} AI-proposed entities", hyp.len()), "AI hypotheses are inference, not evidence; validating them removes the biggest source of speculative risk in the graph.".to_string()) };
        mk(a, w, 0.5, 0.35, "entities", hyp.iter().take(20).map(|e| e.id.clone()).collect());
    }

    // 5) High/critical case — verifying the top actor is the highest-value move.
    if let Some(top) = risk.assessments.iter().filter(|a| a.risk_score >= 0.6).max_by(|a, b| a.risk_score.partial_cmp(&b.risk_score).unwrap()) {
        let (a, w) = if pt { (format!("Verificar {} de maior risco \"{}\" ({:.2})", l.actor, top.entity_label, top.risk_score), "A entidade de maior risco puxa a pontuação do caso; confirmá-la ou descartá-la é o que mais move a decisão.".to_string()) }
            else if es { (format!("Verificar {} de mayor riesgo \"{}\" ({:.2})", l.actor, top.entity_label, top.risk_score), "La entidad de mayor riesgo impulsa la puntuación del caso; confirmarla o descartarla es lo que más mueve la decisión.".to_string()) }
            else { (format!("Verify the highest-risk {} \"{}\" ({:.2})", l.actor, top.entity_label, top.risk_score), "The top-risk entity drives the case score; confirming or clearing it moves the decision the most.".to_string()) };
        mk(a, w, 0.55, 0.3, "graph", vec![top.entity_id.clone()]);
    }

    acts.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
    acts.truncate(6);
    acts
}

/// Render assessments as a Markdown "Assessment" section (localized).
pub fn to_markdown(items: &[Assessment], lang: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    let (head, intro, conf_l, basis_l, ev_l, act_l) = match lang {
        "pt" => ("## Avaliação", "_Dados → informação → inteligência. Cada julgamento declara sua confiança e a evidência por trás. A IA apoia decisões; não decide._", "Confiança", "base", "Evidência", "Ação"),
        "es" => ("## Evaluación", "_Datos → información → inteligencia. Cada juicio declara su confianza y la evidencia detrás. La IA apoya decisiones; no decide._", "Confianza", "base", "Evidencia", "Acción"),
        _ => ("## Assessment", "_Data → information → intelligence. Each judgment states its confidence and the evidence behind it. The AI supports decisions; it does not decide._", "Confidence", "basis", "Evidence", "Action"),
    };
    let mut s = format!("{head}\n\n{intro}\n\n");
    for (i, a) in items.iter().enumerate() {
        s.push_str(&format!("**{}. {}**  \n_{}: {:.0}% · {}: {}_  \n", i + 1, a.statement, conf_l, a.confidence * 100.0, basis_l, a.basis));
        if !a.evidence.is_empty() {
            s.push_str(&format!("{}: {}.  \n", ev_l, a.evidence.join("; ")));
        }
        s.push_str(&format!("{}: {}\n\n", act_l, a.action));
    }
    s
}

/// Render the ranked next-best-actions as a Markdown section (localized).
pub fn nba_to_markdown(items: &[NextAction], lang: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    let (head, intro, unc_l, eff_l, prio_l) = match lang {
        "pt" => ("## Próximas melhores ações", "_Ordenadas por quanto cada passo reduz a incerteza por unidade de esforço._", "Incerteza", "esforço", "prioridade"),
        "es" => ("## Próximas mejores acciones", "_Ordenadas por cuánto reduce cada paso la incertidumbre por unidad de esfuerzo._", "Incertidumbre", "esfuerzo", "prioridad"),
        _ => ("## Next best actions", "_Ranked by how much each step reduces uncertainty per unit of effort._", "Uncertainty", "effort", "priority"),
    };
    let mut s = format!("{head}\n\n{intro}\n\n");
    for (i, a) in items.iter().enumerate() {
        s.push_str(&format!("**{}. {}**  \n_{} ↓ {:.0}% · {} {:.0}% · {} {:.0}%_  \n{}\n\n",
            i + 1, a.action, unc_l, a.uncertainty_reduction * 100.0, eff_l, a.effort * 100.0, prio_l, a.priority * 100.0, a.why));
    }
    s
}
