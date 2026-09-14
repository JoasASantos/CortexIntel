//! Transforms — pluggable enrichment steps that take a seed
//! entity and return new entities/relationships. Transforms execute in one of
//! three runtimes:
//!
//!   * `python`  — an inline Python 3 script (run via `python3`).
//!   * `rust`    — an inline std-only Rust program (compiled once with `rustc`,
//!                 cached by content hash, then executed).
//!   * `command` — an external executable already on disk.
//!
//! I/O contract: the transform receives JSON on stdin
//!   {"input": {...seed...}, "params": {...}, "api_key": "<or empty>"}
//! and must print JSON on stdout
//!   {"entities": [{"kind","label","attributes"}], "relationships": [{"source","type","target","confidence"}]}
//!
//! A curated catalog groups public-service transforms by category (cyber,
//! journalism, hr, investigative, business); installing one drops a manifest
//! into `~/.cortexintel/transforms/` and (if it needs a key) prompts for it.

use crate::{keys, store};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transform {
    pub id: String,
    pub name: String,
    pub category: String, // cyber | journalism | hr | investigative | business
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub service: String, // public service name (for the API key), if any
    #[serde(default)]
    pub requires_api_key: bool,
    /// Entity kinds this transform accepts (empty = any).
    #[serde(default)]
    pub input_kinds: Vec<String>,
    pub runtime: String, // python | rust | command
    /// Inline source (python/rust) or executable path (command).
    pub entrypoint: String,
    /// Legal/ethics notice shown before install/run (e.g. LGPD/GDPR).
    #[serde(default)]
    pub disclaimer: String,
    #[serde(default)]
    pub enabled: bool,
}

fn dir() -> std::path::PathBuf {
    store::base_dir().join("transforms")
}

pub fn list() -> Vec<Transform> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir()) {
        for e in rd.flatten() {
            if e.path().extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(e.path()) {
                if let Ok(t) = serde_json::from_str::<Transform>(&s) {
                    out.push(t);
                }
            }
        }
    }
    out.sort_by(|a, b| (a.category.clone(), a.name.clone()).cmp(&(b.category.clone(), b.name.clone())));
    out
}

pub fn install_manifest(mut t: Transform) -> Result<Transform> {
    if t.id.trim().is_empty() {
        t.id = format!("tf-{}", uuid::Uuid::new_v4().simple());
    }
    if !t.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(anyhow!("invalid transform id"));
    }
    t.enabled = true;
    store::write_json(&dir().join(format!("{}.json", t.id)), &t)?;
    Ok(t)
}

/// Install a curated catalog transform by its catalog id.
pub fn install_from_catalog(catalog_id: &str) -> Result<Transform> {
    let entry = catalog()
        .into_iter()
        .find(|t| t.id == catalog_id)
        .ok_or_else(|| anyhow!("unknown catalog transform '{catalog_id}'"))?;
    // Reset id so installed copy gets its own file keyed by catalog id.
    install_manifest(entry)
}

pub fn remove(id: &str) -> Result<()> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(anyhow!("invalid id"));
    }
    std::fs::remove_file(dir().join(format!("{id}.json"))).map_err(|e| anyhow!("cannot remove: {e}"))
}

pub fn set_enabled(id: &str, enabled: bool) -> Result<()> {
    let path = dir().join(format!("{id}.json"));
    let mut t: Transform = serde_json::from_str(&std::fs::read_to_string(&path).map_err(|_| anyhow!("not found"))?)?;
    t.enabled = enabled;
    store::write_json(&path, &t)
}

/// Run an installed transform against a seed input, returning {entities, relationships}.
pub fn run(id: &str, input: serde_json::Value, params: serde_json::Value) -> Result<serde_json::Value> {
    let t = list().into_iter().find(|t| t.id == id).ok_or_else(|| anyhow!("transform not found"))?;
    if !t.enabled {
        return Err(anyhow!("transform is disabled"));
    }
    let api_key = if t.requires_api_key {
        keys::get(&t.service).ok_or_else(|| anyhow!("missing API key for service '{}'; add it in Settings → API Keys", t.service))?
    } else {
        String::new()
    };
    let payload = serde_json::json!({ "input": input, "params": params, "api_key": api_key });
    let stdin_data = serde_json::to_vec(&payload)?;

    let out = match t.runtime.as_str() {
        "python" => run_python(&t.entrypoint, &stdin_data, &t.service, &api_key)?,
        "rust" => run_rust(&t.entrypoint, &stdin_data, &t.service, &api_key)?,
        "command" => run_command(&t.entrypoint, &stdin_data, &t.service, &api_key)?,
        other => return Err(anyhow!("unknown runtime '{other}'")),
    };

    let parsed: serde_json::Value = crate::llm::extract_json(&out)
        .with_context(|| format!("transform '{}' did not return valid JSON", t.name))?;
    Ok(parsed)
}

fn common_env(cmd: &mut Command, service: &str, api_key: &str) {
    cmd.env("CORTEX_TRANSFORM_SERVICE", service);
    cmd.env("CORTEX_COUNTRY", store::get_settings().country);
    if !api_key.is_empty() {
        cmd.env("TRANSFORM_API_KEY", api_key);
    }
}

fn feed(mut child: std::process::Child, stdin_data: &[u8]) -> Result<String> {
    if let Some(mut si) = child.stdin.take() {
        si.write_all(stdin_data)?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_python(code: &str, stdin_data: &[u8], service: &str, api_key: &str) -> Result<String> {
    let mut cmd = Command::new("python3");
    cmd.arg("-c").arg(code).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    common_env(&mut cmd, service, api_key);
    let child = cmd.spawn().context("spawning python3 — is it installed?")?;
    feed(child, stdin_data)
}

fn run_command(path: &str, stdin_data: &[u8], service: &str, api_key: &str) -> Result<String> {
    let mut cmd = Command::new(path);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    common_env(&mut cmd, service, api_key);
    let child = cmd.spawn().with_context(|| format!("spawning '{path}'"))?;
    feed(child, stdin_data)
}

/// Compile an inline std-only Rust program once (cached by content hash) and run it.
fn run_rust(code: &str, stdin_data: &[u8], service: &str, api_key: &str) -> Result<String> {
    let cache = dir().join(".cache");
    store::ensure_dir(&cache)?;
    let hash = fnv(code);
    let bin = cache.join(format!("rt-{hash:016x}"));
    if !bin.exists() {
        let src = cache.join(format!("rt-{hash:016x}.rs"));
        std::fs::write(&src, code)?;
        let out = Command::new("rustc")
            .args(["-O", "--edition", "2021", "-o"])
            .arg(&bin)
            .arg(&src)
            .output()
            .context("compiling rust transform — is rustc installed?")?;
        if !out.status.success() {
            return Err(anyhow!("rustc: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
    }
    let mut cmd = Command::new(&bin);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    common_env(&mut cmd, service, api_key);
    let child = cmd.spawn().context("running compiled rust transform")?;
    feed(child, stdin_data)
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

// ---------------------------------------------------------------------------
// Curated transform store (enrichment hub), grouped by category.
// ---------------------------------------------------------------------------

/// Public catalog. Entries with `requires_api_key` need a key configured for
/// their `service`. Local (no-key) transforms run out of the box.
pub fn catalog() -> Vec<Transform> {
    vec![
        // ---- CYBER ----
        Transform { id:"cyber.email-to-domain".into(), name:"Email → Domain".into(), category:"cyber".into(),
            description:"Extract the domain from an email account (local, no key).".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["account".into()], runtime:"python".into(),
            entrypoint: PY_EMAIL_TO_DOMAIN.into(), disclaimer:String::new(), enabled:false },
        Transform { id:"cyber.hash-classify".into(), name:"Hash → Type".into(), category:"cyber".into(),
            description:"Classify a hash as MD5/SHA1/SHA256 (local Rust, no key).".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["media".into()], runtime:"rust".into(),
            entrypoint: RS_HASH_CLASSIFY.into(), disclaimer:String::new(), enabled:false },
        Transform { id:"cyber.shodan-host".into(), name:"IP → Shodan Host".into(), category:"cyber".into(),
            description:"Enrich an IP with open ports/services from Shodan.".into(), service:"shodan".into(),
            requires_api_key:true, input_kinds:vec!["ip".into()], runtime:"python".into(),
            entrypoint: PY_SHODAN.into(), disclaimer:String::new(), enabled:false },
        Transform { id:"cyber.virustotal".into(), name:"Hash/URL → VirusTotal".into(), category:"cyber".into(),
            description:"Reputation lookup for a hash or URL via VirusTotal.".into(), service:"virustotal".into(),
            requires_api_key:true, input_kinds:vec!["media".into(),"url".into()], runtime:"python".into(),
            entrypoint: PY_VIRUSTOTAL.into(), disclaimer:String::new(), enabled:false },
        // ---- INVESTIGATIVE ----
        Transform { id:"inv.whois".into(), name:"Domain → WHOIS".into(), category:"investigative".into(),
            description:"Registrant/registrar info via the local `whois` client (no key).".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["domain".into()], runtime:"python".into(),
            entrypoint: PY_WHOIS.into(), disclaimer:String::new(), enabled:false },
        Transform { id:"inv.hibp".into(), name:"Email → Breaches".into(), category:"investigative".into(),
            description:"Check an email against Have I Been Pwned.".into(), service:"hibp".into(),
            requires_api_key:true, input_kinds:vec!["account".into()], runtime:"python".into(),
            entrypoint: PY_HIBP.into(), disclaimer:String::new(), enabled:false },
        // ---- JOURNALISM ----
        Transform { id:"news.github-user".into(), name:"Username → GitHub".into(), category:"journalism".into(),
            description:"Public GitHub profile + repos for a username (no key for public).".into(), service:"github".into(),
            requires_api_key:false, input_kinds:vec!["account".into()], runtime:"python".into(),
            entrypoint: PY_GITHUB.into(), disclaimer:String::new(), enabled:false },
        // ---- HR ----
        Transform { id:"hr.email-normalize".into(), name:"Person → Corporate email".into(), category:"hr".into(),
            description:"Derive likely corporate email patterns from a name + domain (local).".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["person".into()], runtime:"python".into(),
            entrypoint: PY_HR_EMAIL.into(), disclaimer:String::new(), enabled:false },
        // ---- BUSINESS ----
        Transform { id:"biz.opencorporates".into(), name:"Company → Registry".into(), category:"business".into(),
            description:"Look up a company in OpenCorporates.".into(), service:"opencorporates".into(),
            requires_api_key:true, input_kinds:vec!["organization".into()], runtime:"python".into(),
            entrypoint: PY_OPENCORP.into(), disclaimer:String::new(), enabled:false },
        Transform { id:"biz.webhook".into(), name:"Entity → Webhook / API".into(), category:"business".into(),
            description:"POST the selected entity to a webhook/REST endpoint (set params.url). Bring back JSON entities.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec![], runtime:"python".into(),
            entrypoint: PY_WEBHOOK.into(), disclaimer:String::new(), enabled:false },
        // ---- COUNTER-TRAFFICKING (anti-tráfico de pessoas) ----
        Transform { id:"ht.ad-indicators".into(), name:"Anúncio → Indicadores de tráfico".into(), category:"trafficking".into(),
            description:"Analisa texto/atributos de um anúncio ou comunicação e pontua indicadores de tráfico (controle por terceiro, dívida, documentos retidos, movimento entre cidades, sinais de menor, liberdade restrita, códigos/emoji). Local, sem chave.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["url".into(),"report".into(),"communication".into(),"account".into()], runtime:"python".into(),
            entrypoint: PY_HT_AD_INDICATORS.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.phone-pivot".into(), name:"Telefone → Anúncios & contas".into(), category:"trafficking".into(),
            description:"Pivota um telefone/seletor sobre o corpus local de anúncios (CSV do projeto ou CORTEX_HT_ADS_CSV) e retorna anúncios, contas, cidades e datas que reutilizam o mesmo número. Local, sem chave.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["selector".into(),"account".into()], runtime:"python".into(),
            entrypoint: PY_HT_PHONE_PIVOT.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.phone-lookup".into(), name:"Telefone → Operadora / tipo de linha".into(), category:"trafficking".into(),
            description:"Consulta operadora, país e tipo de linha (VoIP/celular) via API numverify-compatível; sinaliza VoIP e número recém-portado como indicador de rotação de chips.".into(), service:"numverify".into(),
            requires_api_key:true, input_kinds:vec!["selector".into()], runtime:"python".into(),
            entrypoint: PY_HT_PHONE_LOOKUP.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.handle-pivot".into(), name:"Handle → Perfis em plataformas".into(), category:"trafficking".into(),
            description:"Verifica a existência de um handle em plataformas públicas (Instagram, X, TikTok, Telegram, OnlyFans, Linktree…) por sondagem HTTP; devolve perfis prováveis para revisão humana. Sem chave.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["account".into()], runtime:"python".into(),
            entrypoint: PY_HT_HANDLE_PIVOT.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.wallet-trace".into(), name:"Carteira cripto → Contrapartes".into(), category:"trafficking".into(),
            description:"Rastreia uma carteira BTC/ETH em explorador público (Blockchair/Blockstream) e devolve contrapartes, volume e datas — para seguir os proventos. Sem chave.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["wallet".into()], runtime:"python".into(),
            entrypoint: PY_HT_WALLET_TRACE.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.route-timeline".into(), name:"Rota → Movimento entre cidades".into(), category:"trafficking".into(),
            description:"Reconstrói a rota de uma vítima/suspeito a partir do atributo route (A>B>C) ou cidades/datas e cria a cadeia de locais com relações moved_to. Local (Rust).".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["victim".into(),"suspect".into(),"person".into(),"device".into()], runtime:"rust".into(),
            entrypoint: RS_HT_ROUTE.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.site-image-match".into(), name:"Imagem → Hotel/quarto (TraffickCam-like)".into(), category:"trafficking".into(),
            description:"Envia o hash/caminho de uma imagem de anúncio a um serviço de identificação de quartos de hotel (endpoint self-hosted ou parceiro, ex.: TraffickCam) e devolve locais candidatos. Requer params.endpoint.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["media".into(),"url".into()], runtime:"python".into(),
            entrypoint: PY_HT_SITE_IMAGE.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.document-check".into(), name:"Documento → Validação (CPF/passaporte)".into(), category:"trafficking".into(),
            description:"Valida formato e dígitos verificadores de CPF/CNPJ/passaporte informados em atributos e sinaliza documento retido/inconsistente (indicador de servidão por dívida). Local, sem chave.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["victim".into(),"person".into(),"suspect".into(),"selector".into(),"organization".into()], runtime:"python".into(),
            entrypoint: PY_HT_DOC_CHECK.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        Transform { id:"ht.referral-package".into(), name:"Entidade → Pacote de encaminhamento".into(), category:"trafficking".into(),
            description:"Monta um pacote de encaminhamento (Disque 100 / Polícia Federal / hotline) com os indicadores observados, locais, seletores e cadeia de custódia como entidade de evidência. Local, sem chave.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec![], runtime:"python".into(),
            entrypoint: PY_HT_REFERRAL.into(), disclaimer:HT_DISCLAIMER.into(), enabled:false },
        // ---- PEOPLE SEARCH ----
        Transform { id:"people.persona".into(), name:"Name/Email → Persona".into(), category:"people".into(),
            description:"People-search: resolve a name/email to a persona (accounts, locations) via a people-search API.".into(), service:"peoplesearch".into(),
            requires_api_key:true, input_kinds:vec!["person".into(),"account".into()], runtime:"python".into(),
            entrypoint: PY_PERSONA.into(),
            disclaimer:"GDPR/LGPD: person searches require a lawful basis and data minimization. Use only for authorized investigations; results are leads, not proof.".into(), enabled:false },
        // ---- KYC / IDENTITY (BR + US) ----
        Transform { id:"kyc.cpf-validate".into(), name:"BR CPF → Validate (local)".into(), category:"kyc".into(),
            description:"Validate a Brazilian CPF's check digits (format only, offline). Does NOT prove identity.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["person".into()], runtime:"python".into(),
            entrypoint: PY_CPF.into(),
            disclaimer:"LGPD: CPF is personal data. Checksum validity ≠ real identity. Lawful basis required.".into(), enabled:false },
        Transform { id:"kyc.ssn-validate".into(), name:"US SSN → Validate (local)".into(), category:"kyc".into(),
            description:"Validate a US SSN's structural format (offline). Does NOT prove identity.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["person".into()], runtime:"python".into(),
            entrypoint: PY_SSN.into(),
            disclaimer:"US privacy: SSN is sensitive PII. Format validity ≠ real identity.".into(), enabled:false },
        Transform { id:"kyc.identity-verify".into(), name:"Document → Identity Verify".into(), category:"kyc".into(),
            description:"Verify whether the person behind a document is real via a KYC provider (country-aware).".into(), service:"kyc_provider".into(),
            requires_api_key:true, input_kinds:vec!["person".into()], runtime:"python".into(),
            entrypoint: PY_KYC.into(),
            disclaimer:"GDPR/LGPD + KYC regulation: identity verification requires explicit lawful basis and provider agreement.".into(), enabled:false },
        Transform { id:"kyc.document-expand".into(), name:"Document → Expand Profile (API)".into(), category:"kyc".into(),
            description:"Take a CPF/RG/CNPJ/SSN/EIN off the entity and query a configurable lookup API (params.endpoint) for the full profile — name, phone, address, email. Since the returned entity shares the same document_id, it merges into the existing person instead of creating a duplicate.".into(), service:"document_lookup".into(),
            requires_api_key:true, input_kinds:vec!["person".into(),"selector".into()], runtime:"python".into(),
            entrypoint: PY_DOC_EXPAND.into(),
            disclaimer:"GDPR/LGPD/CCPA: bulk or automated document lookups require a lawful basis, a signed provider agreement and data-minimization — this only wires the plumbing, it does not grant that authorization.".into(), enabled:false },
        // ---- MEDIA INTELLIGENCE ----
        Transform { id:"media.geoint-ai".into(), name:"Image → AI Geolocation (Gemini)".into(), category:"media".into(),
            description:"Analyze an image with Google Gemini AI to extract geolocation, landmarks, environmental context and visual intelligence. Uses the local gemini CLI (subscription).".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["media".into(),"evidence".into()], runtime:"python".into(),
            entrypoint: PY_GEOINT_AI.into(),
            disclaimer:"AI geolocation is probabilistic — a lead, not ground truth. Always cross-reference with EXIF and corroborating sources.".into(), enabled:false },
        Transform { id:"media.metadata".into(), name:"Media → Metadata (EXIF)".into(), category:"media".into(),
            description:"Extract EXIF/media metadata (camera, GPS, software) via local exiftool.".into(), service:"".into(),
            requires_api_key:false, input_kinds:vec!["media".into()], runtime:"python".into(),
            entrypoint: PY_EXIF.into(),
            disclaimer:"May reveal location/PII embedded in media. Handle per policy.".into(), enabled:false },
        Transform { id:"media.deepfake".into(), name:"Media → Deepfake / manipulation".into(), category:"media".into(),
            description:"Assess whether an image/video is AI-generated or manipulated (deepfake/deepnude) via a detection API.".into(), service:"deepfake_api".into(),
            requires_api_key:true, input_kinds:vec!["media".into()], runtime:"python".into(),
            entrypoint: PY_DEEPFAKE.into(),
            disclaimer:"Detection is probabilistic — a signal, not proof. Never generate or store abusive content; reference by hash only.".into(), enabled:false },
        Transform { id:"media.moderation".into(), name:"Media → Sensitive-content check".into(), category:"media".into(),
            description:"Flag whether media is sensitive/NSFW so it can be gated from view (moderation API).".into(), service:"moderation_api".into(),
            requires_api_key:true, input_kinds:vec!["media".into()], runtime:"python".into(),
            entrypoint: PY_MODERATION.into(),
            disclaimer:"Sensitive content must be handled under strict access controls; do not expose raw material.".into(), enabled:false },
        Transform { id:"media.reverse-image".into(), name:"Image → Reverse image search".into(), category:"media".into(),
            description:"Search where an image appears online via a configurable reverse-image API (SerpAPI Google Lens, TinEye, Bing Visual Search). Set params.endpoint and the API key. Returns the pages/URLs and any linked accounts where the same picture was found.".into(), service:"reverse_image".into(),
            requires_api_key:true, input_kinds:vec!["media".into(),"evidence".into()], runtime:"python".into(),
            entrypoint: PY_REVERSE_IMAGE.into(),
            disclaimer:"Reverse-image hits are leads, not proof of identity. Corroborate before acting; respect each source's terms of use.".into(), enabled:false },
        Transform { id:"media.face-search".into(), name:"Photo → Face search (social profiles)".into(), category:"media".into(),
            description:"Take a person's photo and search face-recognition indexes (e.g. FaceCheck.ID / PimEyes-style) for matching social-media and web profiles. Set params.endpoint and the API key. Returns candidate profile URLs/accounts as new leads to expand.".into(), service:"face_search".into(),
            requires_api_key:true, input_kinds:vec!["media".into(),"evidence".into(),"person".into()], runtime:"python".into(),
            entrypoint: PY_FACE_SEARCH.into(),
            disclaimer:"BIOMETRIC / FACIAL RECOGNITION: highly regulated (GDPR/LGPD/BIPA). Requires an explicit lawful basis and, in many jurisdictions, consent. Matches are probabilistic — never treat a hit as a confirmed identity. Authorized investigations only.".into(), enabled:false },
        Transform { id:"osint.social-profiles".into(), name:"Username / Name → Social profiles".into(), category:"investigative".into(),
            description:"Enumerate accounts a username or person may hold across social networks. Uses the local `sherlock` CLI when installed; otherwise queries a configurable WhatsMyName-style endpoint (params.endpoint). Returns account entities linked back to the person.".into(), service:"social_enum".into(),
            requires_api_key:false, input_kinds:vec!["person".into(),"account".into(),"selector".into()], runtime:"python".into(),
            entrypoint: PY_SOCIAL_PROFILES.into(),
            disclaimer:"Name/handle collisions are common — a found profile is a candidate, not a confirmed match. Verify before attributing to a real person.".into(), enabled:false },
    ]
}

// --- inline transform sources (kept small & std/urllib-only) ---

const PY_EMAIL_TO_DOMAIN: &str = r#"
import sys, json
d=json.load(sys.stdin); lab=(d.get('input') or {}).get('label','')
out={'entities':[],'relationships':[]}
if '@' in lab:
    dom=lab.split('@',1)[1]
    out['entities']=[{'kind':'domain','label':dom,'attributes':{'derived_from':lab}}]
    out['relationships']=[{'source':lab,'type':'uses_domain','target':dom,'confidence':0.9}]
print(json.dumps(out))
"#;

const PY_WHOIS: &str = r#"
import sys, json, subprocess, shutil
d=json.load(sys.stdin); dom=(d.get('input') or {}).get('label','')
out={'entities':[],'relationships':[]}
if shutil.which('whois') and dom:
    try:
        r=subprocess.run(['whois',dom],capture_output=True,text=True,timeout=20).stdout
        reg=[l.split(':',1)[1].strip() for l in r.splitlines() if l.lower().startswith('registrar:')]
        org=[l.split(':',1)[1].strip() for l in r.splitlines() if l.lower().startswith('registrant organization:')]
        attrs={}
        if reg: attrs['registrar']=reg[0]
        if org: attrs['registrant_org']=org[0]
        out['entities']=[{'kind':'organization','label':(org[0] if org else (reg[0] if reg else 'unknown-registrar')),'attributes':attrs}]
        if out['entities']: out['relationships']=[{'source':dom,'type':'registered_via','target':out['entities'][0]['label'],'confidence':0.7}]
    except Exception as e:
        out['error']=str(e)
else:
    out['error']='whois client not available'
print(json.dumps(out))
"#;

const PY_HR_EMAIL: &str = r#"
import sys, json
d=json.load(sys.stdin); inp=d.get('input') or {}; name=inp.get('label',''); dom=(d.get('params') or {}).get('domain','company.com')
out={'entities':[],'relationships':[]}
parts=[p for p in name.lower().replace('.',' ').split() if p]
if len(parts)>=2:
    f,l=parts[0],parts[-1]
    for pat in [f+'.'+l, f[0]+l, f+l, l+'.'+f]:
        em=pat+'@'+dom
        out['entities'].append({'kind':'account','label':em,'attributes':{'pattern':'derived'}})
        out['relationships'].append({'source':name,'type':'possible_email','target':em,'confidence':0.4})
print(json.dumps(out))
"#;

const PY_SHODAN: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); ip=(d.get('input') or {}).get('label',''); key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
out={'entities':[],'relationships':[]}
try:
    u=f'https://api.shodan.io/shodan/host/{ip}?key={key}'
    j=json.load(urllib.request.urlopen(u,timeout=20))
    for p in (j.get('ports') or [])[:50]:
        lab=f'{ip}:{p}'; out['entities'].append({'kind':'service','label':lab,'attributes':{'port':p}})
        out['relationships'].append({'source':ip,'type':'exposes','target':lab,'confidence':0.8})
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_VIRUSTOTAL: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); ind=(d.get('input') or {}).get('label',''); key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
out={'entities':[],'relationships':[]}
try:
    kind='urls' if ind.startswith('http') else 'files'
    req=urllib.request.Request(f'https://www.virustotal.com/api/v3/{kind}/{ind}',headers={'x-apikey':key})
    j=json.load(urllib.request.urlopen(req,timeout=20)); stats=j['data']['attributes']['last_analysis_stats']
    out['entities']=[{'kind':'incident','label':f'VT:{ind[:16]}','attributes':{k:str(v) for k,v in stats.items()}}]
    out['relationships']=[{'source':ind,'type':'reputation','target':out['entities'][0]['label'],'confidence':0.9}]
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_HIBP: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); em=(d.get('input') or {}).get('label',''); key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
out={'entities':[],'relationships':[]}
try:
    req=urllib.request.Request(f'https://haveibeenpwned.com/api/v3/breachedaccount/{em}',headers={'hibp-api-key':key,'user-agent':'CortexIntel'})
    for b in json.load(urllib.request.urlopen(req,timeout=20)):
        nm='breach:'+b.get('Name','?'); out['entities'].append({'kind':'incident','label':nm,'attributes':{'breach':b.get('Name','')}})
        out['relationships'].append({'source':em,'type':'exposed_in','target':nm,'confidence':0.9})
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_GITHUB: &str = r#"
import sys, json, urllib.request
d=json.load(sys.stdin); u=(d.get('input') or {}).get('label',''); u=u.split('@')[0]
out={'entities':[],'relationships':[]}
try:
    j=json.load(urllib.request.urlopen(urllib.request.Request(f'https://api.github.com/users/{u}',headers={'user-agent':'CortexIntel'}),timeout=20))
    lab='gh:'+u; out['entities'].append({'kind':'account','label':lab,'attributes':{'name':j.get('name') or '','company':j.get('company') or '','repos':str(j.get('public_repos',0))}})
    out['relationships'].append({'source':u,'type':'github_profile','target':lab,'confidence':0.7})
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_OPENCORP: &str = r#"
import sys, json, os, urllib.request, urllib.parse
d=json.load(sys.stdin); q=(d.get('input') or {}).get('label',''); key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
out={'entities':[],'relationships':[]}
try:
    u='https://api.opencorporates.com/v0.4/companies/search?q='+urllib.parse.quote(q)+('&api_token='+key if key else '')
    j=json.load(urllib.request.urlopen(u,timeout=20))
    for c in j['results']['companies'][:10]:
        co=c['company']; lab=co['name']; out['entities'].append({'kind':'organization','label':lab,'attributes':{'jurisdiction':co.get('jurisdiction_code',''),'number':co.get('company_number','')}})
        out['relationships'].append({'source':q,'type':'matches_company','target':lab,'confidence':0.6})
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_WEBHOOK: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); inp=d.get('input') or {}; url=(d.get('params') or {}).get('url') or os.environ.get('WEBHOOK_URL','')
out={'entities':[],'relationships':[]}
try:
    if not url: raise Exception('set params.url')
    body=json.dumps({'entity':inp}).encode()
    req=urllib.request.Request(url,data=body,headers={'content-type':'application/json'})
    r=json.load(urllib.request.urlopen(req,timeout=25))
    if isinstance(r,dict) and 'entities' in r: out=r
    else: out['entities']=[{'kind':'incident','label':'webhook:response','attributes':{'raw':str(r)[:200]}}]
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_PERSONA: &str = r#"
import sys, json, os, urllib.request, urllib.parse
d=json.load(sys.stdin); q=(d.get('input') or {}).get('label',''); key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
base=(d.get('params') or {}).get('endpoint','')  # people-search API base returning {results:[{name,emails,usernames,locations}]}
out={'entities':[],'relationships':[]}
try:
    if not base: raise Exception('set params.endpoint to your people-search API base URL')
    u=base+('&' if '?' in base else '?')+'q='+urllib.parse.quote(q)+'&api_key='+key
    j=json.load(urllib.request.urlopen(u,timeout=25))
    for p in (j.get('results') or [])[:10]:
        for em in (p.get('emails') or []): out['entities'].append({'kind':'account','label':em,'attributes':{'via':'peoplesearch'}}); out['relationships'].append({'source':q,'type':'linked_email','target':em,'confidence':0.5})
        for un in (p.get('usernames') or []): out['entities'].append({'kind':'account','label':un,'attributes':{'via':'peoplesearch'}}); out['relationships'].append({'source':q,'type':'linked_username','target':un,'confidence':0.5})
        for loc in (p.get('locations') or []): out['entities'].append({'kind':'location','label':loc,'attributes':{}}); out['relationships'].append({'source':q,'type':'associated_location','target':loc,'confidence':0.4})
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_CPF: &str = r#"
import sys, json, re
d=json.load(sys.stdin); raw=(d.get('input') or {}).get('label',''); cpf=re.sub(r'\D','',raw)
out={'entities':[],'relationships':[]}
def valid(c):
    if len(c)!=11 or c==c[0]*11: return False
    for i in (9,10):
        s=sum(int(c[n])*((i+1)-n) for n in range(i)); dch=(s*10)%11%10
        if dch!=int(c[i]): return False
    return True
if len(cpf)==11:
    v=valid(cpf); lab='cpf-check:'+('valid' if v else 'invalid')
    out['entities']=[{'kind':'incident','label':lab,'attributes':{'checksum':'valid' if v else 'invalid','note':'format only, not identity'}}]
    out['relationships']=[{'source':raw,'type':'document_check','target':lab,'confidence':0.9 if v else 0.5}]
else:
    out['error']='not an 11-digit CPF'
print(json.dumps(out))
"#;

const PY_SSN: &str = r#"
import sys, json, re
d=json.load(sys.stdin); raw=(d.get('input') or {}).get('label',''); ssn=re.sub(r'\D','',raw)
out={'entities':[],'relationships':[]}
def valid(s):
    if len(s)!=9: return False
    a,b,c=s[:3],s[3:5],s[5:]
    if a in ('000','666') or a[0]=='9': return False
    if b=='00' or c=='0000': return False
    return True
if len(ssn)==9:
    v=valid(ssn); lab='ssn-check:'+('valid' if v else 'invalid')
    out['entities']=[{'kind':'incident','label':lab,'attributes':{'format':'valid' if v else 'invalid','note':'format only, not identity'}}]
    out['relationships']=[{'source':raw,'type':'document_check','target':lab,'confidence':0.8 if v else 0.4}]
else:
    out['error']='not a 9-digit SSN'
print(json.dumps(out))
"#;

const PY_KYC: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); inp=d.get('input') or {}; key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
base=(d.get('params') or {}).get('endpoint',''); country=os.environ.get('CORTEX_COUNTRY','')
out={'entities':[],'relationships':[]}
try:
    if not base: raise Exception('set params.endpoint to your KYC provider verify URL')
    body=json.dumps({'name':inp.get('label',''),'attributes':inp.get('attributes',{}),'country':country}).encode()
    req=urllib.request.Request(base,data=body,headers={'authorization':'Bearer '+key,'content-type':'application/json'})
    j=json.load(urllib.request.urlopen(req,timeout=25))
    verdict=j.get('verdict') or j.get('status') or 'unknown'; lab='kyc:'+str(verdict)
    out['entities']=[{'kind':'incident','label':lab,'attributes':{k:str(v) for k,v in j.items() if k in ('verdict','status','score','match')}}]
    out['relationships']=[{'source':inp.get('label',''),'type':'identity_verified','target':lab,'confidence':0.8}]
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

// Queries a configurable document-lookup provider (params.endpoint) with a
// CPF/RG/CNPJ/SSN/EIN pulled off the seed entity, and maps back whatever
// profile fields it returns (name/phone/address/email) onto a `person`
// entity carrying the SAME document_id. Because extract.rs folds document_id
// into the entity's dedup_key, upsert_entity merges this straight into the
// original person instead of spawning a duplicate node — the "expand" the
// operator asked for is just: this transform's output entity IS the original,
// enriched.
const PY_DOC_EXPAND: &str = r#"
import sys, json, os, urllib.request, urllib.parse
d=json.load(sys.stdin); inp=d.get('input') or {}; attrs=inp.get('attributes') or {}
key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
base=(d.get('params') or {}).get('endpoint','')
doc=attrs.get('document_id') or attrs.get('cpf') or attrs.get('rg') or attrs.get('cnpj') or attrs.get('ssn') or attrs.get('ein') or ''
if not doc:
    # Run directly on the document/selector node itself (no document_id
    # attribute to read) — its label IS the number when it looks like one.
    lbl = inp.get('label', '')
    if sum(c.isdigit() for c in lbl) >= 9:
        doc = lbl
out={'entities':[],'relationships':[]}
try:
    if not base: raise Exception('set params.endpoint to your document-lookup provider URL')
    if not doc: raise Exception('seed entity has no document_id/cpf/rg/cnpj/ssn/ein attribute to look up')
    sep = '&' if '?' in base else '?'
    url = base + sep + urllib.parse.urlencode({'document': doc})
    req = urllib.request.Request(url, headers={'authorization': 'Bearer ' + key})
    j = json.load(urllib.request.urlopen(req, timeout=25))
    name = j.get('name') or j.get('full_name') or j.get('nome') or inp.get('label', '')
    profile = {'document_id': doc}
    for src, dst in [('phone','phone'),('telefone','phone'),('phone_number','phone'),
                      ('address','address'),('endereco','address'),
                      ('email','email'),('birth_date','birth_date'),('data_nascimento','birth_date'),
                      ('city','city'),('cidade','city')]:
        if j.get(src): profile[dst] = str(j[src])
    out['entities'] = [{'kind':'person','label':name,'attributes':profile}]
    out['relationships'] = [{'source':inp.get('label',''),'type':'expanded_from_document','target':name,'confidence':0.85}]
except Exception as e:
    out['error'] = str(e)
print(json.dumps(out))
"#;

const PY_GEOINT_AI: &str = r#"
import sys, json, subprocess, shutil, os, base64, re
d=json.load(sys.stdin); inp=d.get('input') or {}; attrs=inp.get('attributes') or {}
path=attrs.get('path') or inp.get('label','')
out={'entities':[],'relationships':[]}

if not shutil.which('gemini'):
    out['error']='gemini CLI not found — install: npm install -g @anthropic-ai/gemini-cli or see https://github.com/google-gemini/gemini-cli'
    print(json.dumps(out)); sys.exit(0)

if not path or not os.path.isfile(path):
    out['error']='image file not found (set attributes.path to the image location)'
    print(json.dumps(out)); sys.exit(0)

ext=os.path.splitext(path)[1].lower()
if ext not in ('.png','.jpg','.jpeg','.gif','.webp','.bmp','.tiff','.tif'):
    out['error']=f'unsupported image format: {ext}'
    print(json.dumps(out)); sys.exit(0)

prompt=f"""Analyze the image file at the absolute path below for geolocation intelligence (GEOINT).

Image path: {path}

Read the file and examine it visually. Identify ALL possible geolocation signals:
- GPS coordinates from EXIF metadata if present
- Visible landmarks, buildings, monuments, signs
- Language on signs, billboards, license plates
- Architecture style (European, Asian, Latin American, etc.)
- Vegetation type, climate indicators, terrain
- Road markings, traffic signs, driving side
- Sun position / shadow analysis if visible
- Brand names, store chains, utility companies
- Vehicle types, license plate formats
- Cultural indicators (clothing, flags, symbols)

Return ONLY a single valid JSON object with these fields (no markdown, no prose):
{{"latitude": <float or null>, "longitude": <float or null>, "confidence": <0.0 to 1.0>, "location_name": "<best guess location name>", "country": "<country name or null>", "country_code": "<ISO 2-letter or null>", "city": "<city or null>", "region": "<state/province or null>", "landmarks": ["<identified landmark 1>", ...], "environmental_clues": ["<clue 1>", ...], "reasoning": "<brief reasoning chain>", "image_description": "<what the image shows>"}}
"""

try:
    r=subprocess.run(['gemini',prompt],capture_output=True,text=True,timeout=120,
                      cwd=os.path.dirname(path) or '/')
    raw=r.stdout.strip()
    if not raw:
        raw=r.stderr.strip()
    if not raw:
        out['error']='gemini returned empty output'
        print(json.dumps(out)); sys.exit(0)

    # Extract JSON from possible markdown fences or prose
    m=re.search(r'```(?:json)?\s*(\{.*?\})\s*```', raw, re.DOTALL)
    if m: raw=m.group(1)
    else:
        m=re.search(r'(\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\})', raw, re.DOTALL)
        if m: raw=m.group(1)

    j=json.loads(raw)
    lat=j.get('latitude'); lon=j.get('longitude')
    conf=float(j.get('confidence') or 0.5)
    loc_name=j.get('location_name') or 'Unknown location'
    country=j.get('country') or ''
    city=j.get('city') or ''
    region=j.get('region') or ''
    reasoning=j.get('reasoning') or ''
    desc=j.get('image_description') or ''

    # Evidence entity with full analysis
    ev_attrs={'analysis_provider':'gemini','confidence':str(conf),'reasoning':reasoning,'image_description':desc}
    if lat is not None: ev_attrs['latitude']=str(lat)
    if lon is not None: ev_attrs['longitude']=str(lon)
    if country: ev_attrs['country']=country
    if j.get('country_code'): ev_attrs['country_code']=j['country_code']
    if city: ev_attrs['city']=city
    if region: ev_attrs['region']=region
    if j.get('landmarks'): ev_attrs['landmarks']=', '.join(j['landmarks'])
    if j.get('environmental_clues'): ev_attrs['environmental_clues']=', '.join(j['environmental_clues'])

    ev_label='geoint:'+os.path.basename(path)
    out['entities'].append({'kind':'evidence','label':ev_label,'attributes':ev_attrs})
    out['relationships'].append({'source':inp.get('label',''),'type':'ai_geolocation','target':ev_label,'confidence':conf})

    # Location entity if coordinates found
    if lat is not None and lon is not None:
        loc_label=loc_name if loc_name!='Unknown location' else f'{lat:.4f},{lon:.4f}'
        loc_attrs={'latitude':str(lat),'longitude':str(lon),'source':'gemini_geoint','confidence':str(conf)}
        if country: loc_attrs['country']=country
        if city: loc_attrs['city']=city
        if region: loc_attrs['region']=region
        out['entities'].append({'kind':'location','label':loc_label,'attributes':loc_attrs})
        out['relationships'].append({'source':ev_label,'type':'located_at','target':loc_label,'confidence':conf})

    # Landmark entities
    for lm in (j.get('landmarks') or [])[:5]:
        if lm and lm.strip():
            out['entities'].append({'kind':'location','label':lm.strip(),'attributes':{'type':'landmark','source':'gemini_geoint'}})
            out['relationships'].append({'source':ev_label,'type':'near_landmark','target':lm.strip(),'confidence':conf*0.8})
except json.JSONDecodeError:
    out['error']='gemini output was not valid JSON: '+raw[:300]
except subprocess.TimeoutExpired:
    out['error']='gemini analysis timed out (120s)'
except Exception as e:
    out['error']=str(e)
print(json.dumps(out))
"#;

const PY_REVERSE_IMAGE: &str = r#"
import sys, json, os, base64, urllib.request, re
d=json.load(sys.stdin); inp=d.get('input') or {}; attrs=inp.get('attributes') or {}
params=d.get('params') or {}; key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
path=attrs.get('path') or inp.get('label',''); endpoint=params.get('endpoint','')
out={'entities':[],'relationships':[]}
if not endpoint:
    out['error']='set params.endpoint to a reverse-image API (SerpAPI Google Lens, TinEye, Bing Visual Search)'
    print(json.dumps(out)); sys.exit(0)
if not path or not os.path.isfile(path):
    out['error']='image file not found (set attributes.path to the image location)'
    print(json.dumps(out)); sys.exit(0)
try:
    with open(path,'rb') as f: b64=base64.b64encode(f.read()).decode()
    body=json.dumps({'image_b64':b64,'api_key':key,'filename':os.path.basename(path)}).encode()
    req=urllib.request.Request(endpoint,data=body,headers={'Content-Type':'application/json','Authorization':'Bearer '+key})
    j=json.load(urllib.request.urlopen(req,timeout=45))
    # Tolerant walk: collect any dicts that carry a url/link + optional title/source.
    hits=[]
    def walk(x):
        if isinstance(x,dict):
            u=x.get('url') or x.get('link') or x.get('source_url')
            if u: hits.append({'url':u,'title':x.get('title') or x.get('name') or '','source':x.get('source') or x.get('domain') or ''})
            for v in x.values(): walk(v)
        elif isinstance(x,list):
            for v in x: walk(v)
    walk(j)
    seen=set()
    for h in hits[:30]:
        u=h['url']
        if u in seen: continue
        seen.add(u)
        dom=re.sub(r'^https?://','',u).split('/')[0]
        lab=(h['title'] or dom)[:80]
        out['entities'].append({'kind':'url','label':u,'attributes':{'title':lab,'source':h['source'] or 'reverse_image','found_via':'reverse_image'}})
        out['relationships'].append({'source':inp.get('label',''),'type':'image_appears_at','target':u,'confidence':0.5})
    if not hits: out['error']='no matches returned by endpoint'
except Exception as e:
    out['error']=str(e)
print(json.dumps(out))
"#;

const PY_FACE_SEARCH: &str = r#"
import sys, json, os, base64, urllib.request, re
d=json.load(sys.stdin); inp=d.get('input') or {}; attrs=inp.get('attributes') or {}
params=d.get('params') or {}; key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
path=attrs.get('path') or ''; endpoint=params.get('endpoint','')
out={'entities':[],'relationships':[]}
if not endpoint:
    out['error']='set params.endpoint to a face-search API (FaceCheck.ID / PimEyes-style)'
    print(json.dumps(out)); sys.exit(0)
if not path or not os.path.isfile(path):
    out['error']='face image not found (set attributes.path to the photo of the person)'
    print(json.dumps(out)); sys.exit(0)
try:
    with open(path,'rb') as f: b64=base64.b64encode(f.read()).decode()
    body=json.dumps({'image_b64':b64,'api_key':key}).encode()
    req=urllib.request.Request(endpoint,data=body,headers={'Content-Type':'application/json','Authorization':'Bearer '+key})
    j=json.load(urllib.request.urlopen(req,timeout=60))
    SOCIAL=('instagram','facebook','twitter','x.com','tiktok','linkedin','vk.com','youtube','telegram','threads','pinterest','reddit')
    hits=[]
    def walk(x):
        if isinstance(x,dict):
            u=x.get('url') or x.get('link') or x.get('profile')
            if u: hits.append({'url':u,'score':x.get('score') or x.get('confidence') or 0})
            for v in x.values(): walk(v)
        elif isinstance(x,list):
            for v in x: walk(v)
    walk(j)
    seen=set()
    for h in hits[:25]:
        u=h['url']
        if u in seen: continue
        seen.add(u)
        try: conf=min(1.0,float(h['score'])/(100.0 if float(h['score'])>1 else 1.0))
        except Exception: conf=0.4
        dom=re.sub(r'^https?://','',u).split('/')[0].lower()
        is_social=any(s in dom for s in SOCIAL)
        kind='account' if is_social else 'url'
        out['entities'].append({'kind':kind,'label':u,'attributes':{'platform':dom,'source':'face_search','match_confidence':str(round(conf,2))}})
        out['relationships'].append({'source':inp.get('label',''),'type':'possible_profile','target':u,'confidence':conf})
    if not hits: out['error']='no face matches returned by endpoint'
except Exception as e:
    out['error']=str(e)
print(json.dumps(out))
"#;

const PY_SOCIAL_PROFILES: &str = r#"
import sys, json, os, subprocess, shutil, urllib.request, urllib.parse, re
d=json.load(sys.stdin); inp=d.get('input') or {}; params=d.get('params') or {}
name=inp.get('label',''); endpoint=params.get('endpoint','')
out={'entities':[],'relationships':[]}
# Derive a username: explicit handle wins, else slug the name.
handle=(inp.get('attributes') or {}).get('username') or params.get('username') or name
handle=re.sub(r'[^A-Za-z0-9_.-]','',handle.strip().replace(' ','')) or name
urls=[]
if shutil.which('sherlock') and handle:
    try:
        r=subprocess.run(['sherlock',handle,'--print-found','--timeout','10','--no-color'],capture_output=True,text=True,timeout=180)
        for line in (r.stdout or '').splitlines():
            m=re.search(r'(https?://\S+)',line)
            if m: urls.append(m.group(1).rstrip('.,'))
    except Exception as e:
        out['error']='sherlock failed: '+str(e)
elif endpoint and handle:
    try:
        u=endpoint+('&' if '?' in endpoint else '?')+'username='+urllib.parse.quote(handle)
        j=json.load(urllib.request.urlopen(u,timeout=45))
        def walk(x):
            if isinstance(x,dict):
                v=x.get('url') or x.get('link')
                if v: urls.append(v)
                for w in x.values(): walk(w)
            elif isinstance(x,list):
                for w in x: walk(w)
        walk(j)
    except Exception as e:
        out['error']=str(e)
else:
    out['error']='install the `sherlock` CLI (pip install sherlock-project) or set params.endpoint to a WhatsMyName-style API'
seen=set()
for u in urls[:40]:
    if u in seen: continue
    seen.add(u)
    dom=re.sub(r'^https?://','',u).split('/')[0].lower()
    out['entities'].append({'kind':'account','label':u,'attributes':{'platform':dom,'username':handle,'source':'social_enum'}})
    out['relationships'].append({'source':name,'type':'possible_account','target':u,'confidence':0.45})
print(json.dumps(out))
"#;

const PY_EXIF: &str = r#"
import sys, json, subprocess, shutil, os
d=json.load(sys.stdin); inp=d.get('input') or {}; path=(inp.get('attributes') or {}).get('path') or inp.get('label','')
out={'entities':[],'relationships':[]}
if shutil.which('exiftool') and os.path.exists(path):
    try:
        j=json.loads(subprocess.run(['exiftool','-json',path],capture_output=True,text=True,timeout=30).stdout)[0]
        keep={k:str(v) for k,v in j.items() if k in ('Make','Model','Software','CreateDate','GPSLatitude','GPSLongitude','MIMEType','FileType')}
        lab='meta:'+os.path.basename(path); out['entities']=[{'kind':'evidence','label':lab,'attributes':keep}]
        out['relationships']=[{'source':inp.get('label',''),'type':'has_metadata','target':lab,'confidence':0.95}]
        if 'GPSLatitude' in keep: out['entities'].append({'kind':'location','label':keep.get('GPSLatitude','')+','+keep.get('GPSLongitude',''),'attributes':{'from':'exif'}})
    except Exception as e: out['error']=str(e)
else:
    out['error']='exiftool not installed or path missing (attributes.path)'
print(json.dumps(out))
"#;

const PY_DEEPFAKE: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); inp=d.get('input') or {}; key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
base=(d.get('params') or {}).get('endpoint',''); ref=(inp.get('attributes') or {}).get('url') or inp.get('label','')
out={'entities':[],'relationships':[]}
try:
    if not base: raise Exception('set params.endpoint to your deepfake-detection API')
    body=json.dumps({'media_ref':ref}).encode()
    req=urllib.request.Request(base,data=body,headers={'authorization':'Bearer '+key,'content-type':'application/json'})
    j=json.load(urllib.request.urlopen(req,timeout=30))
    score=j.get('deepfake_score', j.get('score','?')); lab='deepfake:%.2f'%float(score) if isinstance(score,(int,float)) else 'deepfake:?'
    out['entities']=[{'kind':'incident','label':lab,'attributes':{'deepfake_score':str(score),'nsfw':str(j.get('nsfw',''))}}]
    out['relationships']=[{'source':inp.get('label',''),'type':'authenticity_check','target':lab,'confidence':0.7}]
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const PY_MODERATION: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); inp=d.get('input') or {}; key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
base=(d.get('params') or {}).get('endpoint',''); ref=(inp.get('attributes') or {}).get('url') or inp.get('label','')
out={'entities':[],'relationships':[]}
try:
    if not base: raise Exception('set params.endpoint to your moderation API')
    req=urllib.request.Request(base,data=json.dumps({'media_ref':ref}).encode(),headers={'authorization':'Bearer '+key,'content-type':'application/json'})
    j=json.load(urllib.request.urlopen(req,timeout=30))
    sens=j.get('sensitive', j.get('flagged','?')); lab='sensitive:'+str(sens)
    out['entities']=[{'kind':'incident','label':lab,'attributes':{k:str(v) for k,v in j.items() if k in ('sensitive','flagged','categories')}}]
    out['relationships']=[{'source':inp.get('label',''),'type':'content_moderation','target':lab,'confidence':0.7}]
except Exception as e: out['error']=str(e)
print(json.dumps(out))
"#;

const RS_HASH_CLASSIFY: &str = r#"
use std::io::Read;
fn main(){
  let mut s=String::new(); std::io::stdin().read_to_string(&mut s).ok();
  let label = s.split("\"label\"").nth(1).and_then(|x| x.split('"').nth(1)).unwrap_or("").to_string();
  let n = label.len();
  let kind = if n==32 {"md5"} else if n==40 {"sha1"} else if n==64 {"sha256"} else {"unknown"};
  let hex = !label.is_empty() && label.chars().all(|c| c.is_ascii_hexdigit());
  let out = format!("{{\"entities\":[{{\"kind\":\"incident\",\"label\":\"hashtype:{}\",\"attributes\":{{\"algo\":\"{}\",\"is_hex\":\"{}\"}}}}],\"relationships\":[{{\"source\":\"{}\",\"type\":\"classified_as\",\"target\":\"hashtype:{}\",\"confidence\":0.9}}]}}", kind, kind, hex, label, kind);
  println!("{}", out);
}
"#;

// ---------------------------------------------------------------------------
// Counter-trafficking transforms
// ---------------------------------------------------------------------------
const HT_DISCLAIMER: &str = "Uso exclusivo para investigação legítima anti-tráfico (polícia, MP, hotlines, ONGs com base legal). Indicadores são pistas que exigem corroboração — nunca prova. Abordagem centrada na vítima: não exponha identidade de vítimas além da necessidade operacional (LGPD/GDPR).";

const PY_HT_AD_INDICATORS: &str = r#"
import sys, json, re
d=json.load(sys.stdin); inp=d.get('input') or {}; a=inp.get('attributes') or {}
text=' '.join(str(v) for v in [inp.get('label',''), a.get('ad_text',''), a.get('notes',''), a.get('report_category',''), a.get('indicators','')]).lower()
age=str(a.get('age_stated','')).strip()
RULES=[
 ('third_party_control',0.9,[r'ag[êe]ncia',r'agenda (pela|com a) recep',r'assessor',r'gerente',r'ger[êe]ncia',r'meu tio',r'agenda com',r'recep[çc][ãa]o']),
 ('minor_indicator',1.0,[r'novinha',r'rec[ée]m sa[ií]da do col[ée]gio',r'primeira vez',r'\bnew\b.*\btown\b',r'fresh',r'young']),
 ('movement_between_cities',0.7,[r'rec[ée]m chegad',r'nova na cidade',r'passando por',r'chegando em',r'poucos dias',r'2 dias',r'[úu]ltima semana',r'antes de cruzar',r'viagem']),
 ('restricted_freedom',0.9,[r'n[ãa]o atende fora',r's[óo] no local',r'n[ãa]o sai',r'hor[áa]rio controlad',r'24h',r'dia todo']),
 ('debt_bondage',0.9,[r'meta',r'd[íi]vida',r'valores fixad',r'pela casa',r'passagem (e|paga)']),
 ('cash_only',0.4,[r'dinheiro na m[ãa]o',r'cart[ãa]o n[ãa]o',r's[óo] dinheiro',r'cash only']),
 ('phone_rotation',0.6,[r'novo n[úu]mero',r'mesma ag[êe]ncia']),
 ('recruitment_bait',0.8,[r'sem experi[êe]ncia',r'passagem e hospedagem',r'hostess',r'modelo',r'exterior',r'euro']),
 ('shared_lodging',0.6,[r'mesmo hotel',r'duas amigas',r'hotel pr[óo]prio',r'apartamento',r'pousada']),
]
hits=[]; score=0.0
for name,w,pats in RULES:
    m=[p for p in pats if re.search(p,text)]
    if m: hits.append({'indicator':name,'weight':w,'matched':m[:3]}); score+=w
try:
    if age and int(float(age))<=18: hits.append({'indicator':'age_at_threshold','weight':0.8,'matched':[age]}); score+=0.8
except Exception: pass
score=min(1.0, score/3.5)
band='critical' if score>=0.8 else 'high' if score>=0.55 else 'medium' if score>=0.3 else 'low'
out={'entities':[],'relationships':[]}
lab=inp.get('label','')
if hits:
    inc=f"indicadores:{lab[:40]}"
    out['entities'].append({'kind':'incident','label':inc,'attributes':{'score':f'{score:.2f}','band':band,'indicators':', '.join(h['indicator'] for h in hits),'detail':json.dumps(hits,ensure_ascii=False)[:800],'method':'ht.ad-indicators (regras lexicais pt/en)'}})
    out['relationships'].append({'source':lab,'type':'shows_indicators','target':inc,'confidence':round(0.5+score/2,2)})
    for h in hits:
        if h['weight']>=0.9:
            il=f"indicador:{h['indicator']}"
            out['entities'].append({'kind':'incident','label':il,'attributes':{'weight':str(h['weight'])}})
            out['relationships'].append({'source':inc,'type':'includes','target':il,'confidence':0.7})
else:
    out['entities'].append({'kind':'incident','label':f'indicadores:{lab[:40]}','attributes':{'score':'0.00','band':'low','indicators':'nenhum indicador lexical'}})
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_PHONE_PIVOT: &str = r#"
import sys, json, os, csv, re, glob
d=json.load(sys.stdin); inp=d.get('input') or {}; params=d.get('params') or {}
label=inp.get('label',''); a=inp.get('attributes') or {}
digits=lambda s: re.sub(r'\D','',str(s or ''))
key=digits(label) if inp.get('kind')!='account' else ''
handle=label.lower() if inp.get('kind')=='account' else ''
paths=[p for p in [params.get('corpus'), os.environ.get('CORTEX_HT_ADS_CSV')] if p]
for base in [os.getcwd(), os.path.expanduser('~/.cortexintel/uploads'), os.path.join(os.getcwd(),'demos'), os.path.join(os.getcwd(),'scenarios','human-trafficking')]:
    paths+=glob.glob(os.path.join(base,'*trafficking*ads*.csv'))
out={'entities':[],'relationships':[]}; seen=set(); rows=0
for p in dict.fromkeys(paths):
    try:
        with open(p,newline='',encoding='utf-8') as f:
            for r in csv.DictReader(f):
                rows+=1
                ph=digits(r.get('phone_number') or r.get('phone') or ''); acc=(r.get('account_id') or r.get('username') or '').lower()
                if not ((key and ph and (ph.endswith(key[-8:]) or key.endswith(ph[-8:]))) or (handle and acc==handle)): continue
                url=r.get('full_url') or r.get('url') or r.get('report_id') or ''
                city=r.get('city',''); ts=r.get('timestamp') or r.get('posted_at') or ''
                if url and url not in seen:
                    seen.add(url)
                    out['entities'].append({'kind':'url','label':url,'attributes':{'city':city,'timestamp':ts,'report_category':r.get('report_category',''),'age_stated':r.get('age_stated',''),'source':os.path.basename(p)}})
                    out['relationships'].append({'source':label,'type':'listed_in_ad','target':url,'confidence':0.85})
                    if city and city not in seen:
                        seen.add(city); out['entities'].append({'kind':'location','label':city,'attributes':{'latitude':r.get('latitude',''),'longitude':r.get('longitude','')}})
                    if city: out['relationships'].append({'source':url,'type':'posted_in','target':city,'confidence':0.7})
                if acc and acc!=handle and acc not in seen:
                    seen.add(acc); out['entities'].append({'kind':'account','label':acc,'attributes':{'platform':r.get('provider_name','')}})
                    out['relationships'].append({'source':label,'type':'shared_with_account','target':acc,'confidence':0.75})
                if ph and key and ph!=key and ph not in seen and handle:
                    seen.add(ph); out['entities'].append({'kind':'selector','label':'+'+ph,'attributes':{'type':'phone'}})
                    out['relationships'].append({'source':label,'type':'uses_phone','target':'+'+ph,'confidence':0.7})
    except Exception as e:
        out.setdefault('warnings',[]).append(f'{p}: {e}')
n=len([e for e in out['entities'] if e['kind']=='url'])
cities=sorted({e['label'] for e in out['entities'] if e['kind']=='location'})
out['entities'].append({'kind':'incident','label':f'pivot:{label[:30]}','attributes':{'ads_found':str(n),'cities':', '.join(cities),'rows_scanned':str(rows),'note':'telefone reutilizado em múltiplos anúncios/cidades é indicador forte de controle por terceiro' if n>=2 else 'sem reutilização observada'}})
out['relationships'].append({'source':label,'type':'pivot_summary','target':f'pivot:{label[:30]}','confidence':0.6})
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_PHONE_LOOKUP: &str = r#"
import sys, json, os, re, urllib.request, urllib.parse
d=json.load(sys.stdin); inp=d.get('input') or {}; key=d.get('api_key') or os.environ.get('TRANSFORM_API_KEY','')
num=re.sub(r'\D','',inp.get('label','')); out={'entities':[],'relationships':[]}
try:
    base=(d.get('params') or {}).get('endpoint') or os.environ.get('NUMVERIFY_URL','http://apilayer.net/api/validate')
    j=json.load(urllib.request.urlopen(f'{base}?access_key={key}&number={num}',timeout=20))
    if not j.get('valid'): raise Exception('número inválido ou não encontrado')
    lt=(j.get('line_type') or '').lower(); carrier=j.get('carrier') or '?'
    lab=f"linha:{num[-8:]}"
    flags=[]
    if 'voip' in lt: flags.append('voip_line')
    out['entities'].append({'kind':'incident','label':lab,'attributes':{'carrier':carrier,'line_type':lt,'country':j.get('country_name',''),'location':j.get('location',''),'indicators':', '.join(flags) or 'nenhum'}})
    out['relationships'].append({'source':inp.get('label',''),'type':'carrier_info','target':lab,'confidence':0.8})
    if carrier and carrier!='?':
        out['entities'].append({'kind':'organization','label':carrier,'attributes':{'role':'operadora'}})
        out['relationships'].append({'source':inp.get('label',''),'type':'served_by','target':carrier,'confidence':0.7})
except Exception as e: out['error']=str(e)
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_HANDLE_PIVOT: &str = r#"
import sys, json, urllib.request, concurrent.futures
d=json.load(sys.stdin); inp=d.get('input') or {}; h=inp.get('label','').strip().lstrip('@')
SITES={'instagram':'https://www.instagram.com/{}/','x':'https://x.com/{}','tiktok':'https://www.tiktok.com/@{}','telegram':'https://t.me/{}','onlyfans':'https://onlyfans.com/{}','linktree':'https://linktr.ee/{}','facebook':'https://www.facebook.com/{}','kwai':'https://www.kwai.com/@{}','privacy':'https://privacy.com.br/profile/{}'}
out={'entities':[],'relationships':[]}
def probe(item):
    site,tpl=item; url=tpl.format(h)
    try:
        req=urllib.request.Request(url,headers={'User-Agent':'Mozilla/5.0 (CortexIntel OSINT)'},method='HEAD')
        r=urllib.request.urlopen(req,timeout=8); return site,url,r.status
    except urllib.error.HTTPError as e: return site,url,e.code
    except Exception: return site,url,0
if h:
    with concurrent.futures.ThreadPoolExecutor(6) as ex:
        for site,url,st in ex.map(probe,SITES.items()):
            if st in (200,301,302):
                out['entities'].append({'kind':'account','label':f'{h}@{site}','attributes':{'platform':site,'profile_url':url,'http_status':str(st),'note':'existência provável — confirmar manualmente'}})
                out['relationships'].append({'source':inp.get('label',''),'type':'same_handle_on','target':f'{h}@{site}','confidence':0.55})
if not out['entities']: out['entities'].append({'kind':'incident','label':f'handle:{h}','attributes':{'result':'nenhum perfil público respondeu (ou bloqueio de bot)'}})
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_WALLET_TRACE: &str = r#"
import sys, json, urllib.request
d=json.load(sys.stdin); inp=d.get('input') or {}; addr=inp.get('label','').strip(); out={'entities':[],'relationships':[]}
def get(u):
    req=urllib.request.Request(u,headers={'User-Agent':'CortexIntel'}); return json.load(urllib.request.urlopen(req,timeout=25))
try:
    if addr.startswith('0x') and len(addr)==42:
        j=get(f'https://api.blockchair.com/ethereum/dashboards/address/{addr}?limit=25')
        a=j['data'][addr]['address']; calls=j['data'][addr].get('calls',[])
        out['entities'].append({'kind':'incident','label':f'chain:{addr[:10]}','attributes':{'chain':'ethereum','balance_wei':str(a.get('balance','')),'tx_count':str(a.get('transaction_count','')),'first_seen':str(a.get('first_seen_receiving','')),'last_seen':str(a.get('last_seen_receiving',''))}})
        out['relationships'].append({'source':addr,'type':'chain_summary','target':f'chain:{addr[:10]}','confidence':0.9})
        for c in calls[:25]:
            other=c.get('recipient') if c.get('sender','').lower()==addr.lower() else c.get('sender')
            if not other: continue
            out['entities'].append({'kind':'wallet','label':other,'attributes':{'chain':'ethereum'}})
            out['relationships'].append({'source':addr if c.get('sender','').lower()==addr.lower() else other,'type':'transferred_to','target':other if c.get('sender','').lower()==addr.lower() else addr,'confidence':0.85})
    else:
        j=get(f'https://blockstream.info/api/address/{addr}'); st=j.get('chain_stats',{})
        out['entities'].append({'kind':'incident','label':f'chain:{addr[:10]}','attributes':{'chain':'bitcoin','funded_txo_sum':str(st.get('funded_txo_sum','')),'spent_txo_sum':str(st.get('spent_txo_sum','')),'tx_count':str(st.get('tx_count',''))}})
        out['relationships'].append({'source':addr,'type':'chain_summary','target':f'chain:{addr[:10]}','confidence':0.9})
        txs=get(f'https://blockstream.info/api/address/{addr}/txs')
        seen=set()
        for tx in txs[:20]:
            for o in tx.get('vout',[]):
                oa=o.get('scriptpubkey_address')
                if oa and oa!=addr and oa not in seen and len(seen)<25:
                    seen.add(oa); out['entities'].append({'kind':'wallet','label':oa,'attributes':{'chain':'bitcoin','value_sat':str(o.get('value',''))}})
                    out['relationships'].append({'source':addr,'type':'transferred_to','target':oa,'confidence':0.7})
except Exception as e: out['error']=str(e)
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_SITE_IMAGE: &str = r#"
import sys, json, os, urllib.request
d=json.load(sys.stdin); inp=d.get('input') or {}; a=inp.get('attributes') or {}; params=d.get('params') or {}
ep=params.get('endpoint') or os.environ.get('CORTEX_HT_IMAGE_ENDPOINT',''); out={'entities':[],'relationships':[]}
try:
    if not ep: raise Exception('set params.endpoint (serviço de identificação de quartos de hotel, ex.: instância TraffickCam)')
    body=json.dumps({'label':inp.get('label',''),'path':a.get('path',''),'hash':a.get('sha256') or a.get('hash',''),'phash':a.get('perceptual_hash','')}).encode()
    req=urllib.request.Request(ep,data=body,headers={'content-type':'application/json'})
    r=json.load(urllib.request.urlopen(req,timeout=40))
    for m in (r.get('matches') or r.get('candidates') or [])[:10]:
        name=m.get('hotel') or m.get('name') or 'local'; lab=f"{name}"
        out['entities'].append({'kind':'facility','label':lab,'attributes':{'city':m.get('city',''),'latitude':str(m.get('lat','')),'longitude':str(m.get('lon','')),'similarity':str(m.get('score','')),'room':str(m.get('room',''))}})
        out['relationships'].append({'source':inp.get('label',''),'type':'possibly_taken_at','target':lab,'confidence':min(0.9,float(m.get('score',0.5) or 0.5))})
    if not out['entities']: out['entities'].append({'kind':'incident','label':f"imagem:{inp.get('label','')[:24]}",'attributes':{'result':'sem candidatos'}})
except Exception as e: out['error']=str(e)
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_DOC_CHECK: &str = r#"
import sys, json, re
d=json.load(sys.stdin); inp=d.get('input') or {}; a=inp.get('attributes') or {}; out={'entities':[],'relationships':[]}
def cpf_ok(c):
    c=re.sub(r'\D','',c)
    if len(c)!=11 or c==c[0]*11: return False
    for n in (9,10):
        s=sum(int(c[i])*((n+1)-i) for i in range(n)); dv=(s*10%11)%10
        if dv!=int(c[n]): return False
    return True
def cnpj_ok(c):
    c=re.sub(r'\D','',c)
    if len(c)!=14 or c==c[0]*14: return False
    w1=[5,4,3,2,9,8,7,6,5,4,3,2]; w2=[6]+w1
    for w,n in ((w1,12),(w2,13)):
        s=sum(int(c[i])*w[i] for i in range(n)); dv=11-s%11; dv=0 if dv>=10 else dv
        if dv!=int(c[n]): return False
    return True
found=[]
for k,v in a.items():
    kl=k.lower(); v=str(v)
    if kl in('cpf','document_id','documento') and re.sub(r'\D','',v).__len__()==11: found.append(('cpf',v,cpf_ok(v)))
    elif kl in('cnpj',) or (kl=='document_id' and len(re.sub(r'\D','',v))==14): found.append(('cnpj',v,cnpj_ok(v)))
    elif 'passport' in kl or 'passaporte' in kl: found.append(('passport',v,bool(re.match(r'^[A-Z]{1,2}[0-9]{6,8}$',v.strip().upper()))))
flags=[]
ds=str(a.get('doc_status','')).lower()
if ds in('retained','retido','confiscado','withheld'): flags.append('document_retained')
if any(not ok for _,_,ok in found): flags.append('document_invalid_or_forged')
age=a.get('age_stated')
try:
    if age and int(float(age))<18: flags.append('minor_declared')
except Exception: pass
lab=f"documentos:{inp.get('label','')[:30]}"
out['entities'].append({'kind':'incident','label':lab,'attributes':{'checked':'; '.join(f'{t}:{"ok" if ok else "INVÁLIDO"}' for t,_,ok in found) or 'nenhum documento nos atributos','indicators':', '.join(flags) or 'nenhum','note':'documento retido por terceiro é indicador clássico de servidão por dívida (Protocolo de Palermo)'}})
out['relationships'].append({'source':inp.get('label',''),'type':'document_check','target':lab,'confidence':0.8 if flags else 0.5})
print(json.dumps(out,ensure_ascii=False))
"#;

const PY_HT_REFERRAL: &str = r#"
import sys, json, datetime
d=json.load(sys.stdin); inp=d.get('input') or {}; a=inp.get('attributes') or {}
now=datetime.datetime.utcnow().strftime('%Y-%m-%dT%H:%M:%SZ')
ind=[x for x in [a.get('indicators'), a.get('report_category'), a.get('doc_status'), a.get('stage')] if x]
pkg={'referral_to':['Disque 100 (Direitos Humanos)','Polícia Federal — tráfico de pessoas (Lei 13.344/2016)','Núcleo de Enfrentamento ao Tráfico de Pessoas (NETP) do estado'],
     'subject':inp.get('label',''),'kind':inp.get('kind',''),'indicators':ind,'locations':[x for x in [a.get('city'),a.get('location'),a.get('hotel')] if x],
     'selectors':[x for x in [a.get('phone'),a.get('phone_number')] if x],'route':a.get('route',''),'generated_at':now,
     'chain_of_custody':{'source':inp.get('sources',''),'exported_by':'CortexIntel','note':'preservar originais; hash dos arquivos no case.json'},
     'victim_centred':'não confrontar suspeitos; priorizar segurança, consentimento e sigilo da vítima; acionar assistência (CREAS/abrigo)'}
lab=f"encaminhamento:{inp.get('label','')[:30]}"
out={'entities':[{'kind':'evidence','label':lab,'attributes':{'package':json.dumps(pkg,ensure_ascii=False)[:1500],'generated_at':now,'status':'rascunho — revisão humana obrigatória'}}],
     'relationships':[{'source':inp.get('label',''),'type':'referral_package','target':lab,'confidence':0.9}]}
print(json.dumps(out,ensure_ascii=False))
"#;

const RS_HT_ROUTE: &str = r#"
use std::io::Read;
fn get<'a>(s: &'a str, key: &str) -> Option<&'a str> {
  let pat = format!("\"{}\"", key);
  let i = s.find(&pat)? + pat.len();
  let rest = &s[i..];
  let q = rest.find('"')? + 1;
  let rest = &rest[q..];
  let e = rest.find('"')?;
  Some(&rest[..e])
}
fn main(){
  let mut s=String::new(); std::io::stdin().read_to_string(&mut s).ok();
  let label = get(&s,"label").unwrap_or("").to_string();
  let route = get(&s,"route").unwrap_or("").to_string();
  let mut cities: Vec<String> = route.split(|c| c=='>' || c=='→' || c==';' || c==',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
  if cities.is_empty() { if let Some(c) = get(&s,"city") { cities.push(c.to_string()); } }
  let mut ents = Vec::new(); let mut rels = Vec::new();
  for (i,c) in cities.iter().enumerate() {
    ents.push(format!("{{\"kind\":\"location\",\"label\":\"{}\",\"attributes\":{{\"hop\":\"{}\"}}}}", c, i+1));
    if i==0 { rels.push(format!("{{\"source\":\"{}\",\"type\":\"origin\",\"target\":\"{}\",\"confidence\":0.7}}", label, c)); }
    else { rels.push(format!("{{\"source\":\"{}\",\"type\":\"moved_to\",\"target\":\"{}\",\"confidence\":0.75}}", cities[i-1], c)); rels.push(format!("{{\"source\":\"{}\",\"type\":\"seen_in\",\"target\":\"{}\",\"confidence\":0.6}}", label, c)); }
  }
  let n = cities.len();
  ents.push(format!("{{\"kind\":\"incident\",\"label\":\"rota:{}\",\"attributes\":{{\"hops\":\"{}\",\"route\":\"{}\",\"indicator\":\"{}\"}}}}", label.chars().take(30).collect::<String>(), n, cities.join(" > "), if n>=3 {"movimento_multicidade (indicador de transporte/exploração itinerante)"} else {"rota curta"}));
  rels.push(format!("{{\"source\":\"{}\",\"type\":\"route_summary\",\"target\":\"rota:{}\",\"confidence\":0.6}}", label, label.chars().take(30).collect::<String>()));
  println!("{{\"entities\":[{}],\"relationships\":[{}]}}", ents.join(","), rels.join(","));
}
"#;
