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
            requires_api_key:true, input_kinds:vec!["person".into()], runtime:"python".into(),
            entrypoint: PY_DOC_EXPAND.into(),
            disclaimer:"GDPR/LGPD/CCPA: bulk or automated document lookups require a lawful basis, a signed provider agreement and data-minimization — this only wires the plumbing, it does not grant that authorization.".into(), enabled:false },
        // ---- MEDIA INTELLIGENCE ----
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
