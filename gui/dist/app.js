/* ===== CortexIntel GUI (v2) ===== */
"use strict";

// ---------- i18n (EN / PT / ES, switchable in Settings) ----------
// Continuous coverage: static shell strings carry data-i18n / data-i18n-ph in
// index.html; dynamic strings call t("key"). Missing keys fall back to English,
// then to the key itself, so partial coverage never breaks the UI.
const I18N = {
  en: {
    "nav.dashboard":"Dashboard","nav.graph":"Graph","nav.intelligence":"Intelligence","nav.entities":"Entities",
    "nav.timeline":"Timeline","nav.alerts":"Alerts","nav.reports":"Reports","nav.settings":"Settings",
    "set.account":"Account","set.providers":"Providers & Routing","set.datasources":"Data Sources","set.transforms":"Transforms Store",
    "set.keys":"API Keys","set.plugins":"Classifier Plugins","set.project":"Project","set.users":"Users & Access","set.security":"Security","set.language":"Language",
    "btn.run":"Run","btn.askai":"Ask AI","btn.newProject":"New project","btn.fit":"Fit","btn.reset":"Reset","btn.path":"Path","btn.addEntity":"Entity",
    "launcher.open":"Open a recent project or start a new investigation.","launcher.new":"New project","launcher.import":"Import project","launcher.empty":"No projects yet — create your first investigation.",
    "decision.title":"Decision panel","decision.recommended":"Recommended","decision.feasible":"Feasible","decision.highrisk":"High risk","decision.viewGraph":"View in graph","decision.why":"Why","decision.attributedTo":"Analyzed by","decision.none":"Run an analysis to generate decision options.",
    "ingest.title":"Prepare this source","ingest.relevant":"Ingest only what's relevant to this context","ingest.all":"Ingest everything","ingest.detected":"Detected columns","ingest.rows":"rows",
  },
  pt: {
    "nav.dashboard":"Painel","nav.graph":"Grafo","nav.intelligence":"Inteligência","nav.entities":"Entidades",
    "nav.timeline":"Linha do tempo","nav.alerts":"Alertas","nav.reports":"Relatórios","nav.settings":"Ajustes",
    "set.account":"Conta","set.providers":"Provedores & Roteamento","set.datasources":"Fontes de Dados","set.transforms":"Loja de Transforms",
    "set.keys":"Chaves de API","set.plugins":"Plugins de Classificação","set.project":"Projeto","set.users":"Usuários & Acesso","set.security":"Segurança","set.language":"Idioma",
    "btn.run":"Executar","btn.askai":"Perguntar à IA","btn.newProject":"Novo projeto","btn.fit":"Ajustar","btn.reset":"Redefinir","btn.path":"Caminho","btn.addEntity":"Entidade",
    "launcher.open":"Abra um projeto recente ou inicie uma nova investigação.","launcher.new":"Novo projeto","launcher.import":"Importar projeto","launcher.empty":"Nenhum projeto ainda — crie sua primeira investigação.",
    "decision.title":"Painel de decisão","decision.recommended":"Recomendado","decision.feasible":"Viável","decision.highrisk":"Alto risco","decision.viewGraph":"Ver no grafo","decision.why":"Por quê","decision.attributedTo":"Analisado por","decision.none":"Execute uma análise para gerar opções de decisão.",
    "ingest.title":"Preparar esta fonte","ingest.relevant":"Ingerir só o que é relevante para este contexto","ingest.all":"Ingerir tudo","ingest.detected":"Colunas detectadas","ingest.rows":"linhas",
  },
  es: {
    "nav.dashboard":"Panel","nav.graph":"Grafo","nav.intelligence":"Inteligencia","nav.entities":"Entidades",
    "nav.timeline":"Línea de tiempo","nav.alerts":"Alertas","nav.reports":"Informes","nav.settings":"Ajustes",
    "set.account":"Cuenta","set.providers":"Proveedores y Enrutamiento","set.datasources":"Fuentes de Datos","set.transforms":"Tienda de Transforms",
    "set.keys":"Claves de API","set.plugins":"Plugins de Clasificación","set.project":"Proyecto","set.users":"Usuarios y Acceso","set.security":"Seguridad","set.language":"Idioma",
    "btn.run":"Ejecutar","btn.askai":"Preguntar a la IA","btn.newProject":"Nuevo proyecto","btn.fit":"Ajustar","btn.reset":"Restablecer","btn.path":"Ruta","btn.addEntity":"Entidad",
    "launcher.open":"Abre un proyecto reciente o inicia una nueva investigación.","launcher.new":"Nuevo proyecto","launcher.import":"Importar proyecto","launcher.empty":"Aún no hay proyectos — crea tu primera investigación.",
    "decision.title":"Panel de decisión","decision.recommended":"Recomendado","decision.feasible":"Viable","decision.highrisk":"Alto riesgo","decision.viewGraph":"Ver en el grafo","decision.why":"Por qué","decision.attributedTo":"Analizado por","decision.none":"Ejecuta un análisis para generar opciones de decisión.",
    "ingest.title":"Preparar esta fuente","ingest.relevant":"Ingerir solo lo relevante para este contexto","ingest.all":"Ingerir todo","ingest.detected":"Columnas detectadas","ingest.rows":"filas",
  },
};
function detectLang(){ const s=localStorage.getItem("cortex_lang"); if(s&&I18N[s])return s; const n=(navigator.language||"en").slice(0,2).toLowerCase(); return I18N[n]?n:"en"; }
let LANG = detectLang();
function t(key, ...args){ let s=(I18N[LANG]&&I18N[LANG][key]) || I18N.en[key] || key; args.forEach((a,i)=>{ s=s.replace("{"+i+"}",a); }); return s; }
function setLang(l){ if(!I18N[l])return; LANG=l; localStorage.setItem("cortex_lang",l); document.documentElement.lang=l; applyI18n(); try{ applyIcons(); }catch(e){} try{ if(typeof showLauncher==="function" && !document.getElementById("launcher").hidden) showLauncher(); }catch(e){} }
function applyI18n(){
  document.querySelectorAll("[data-i18n]").forEach(e=>{ const k=e.getAttribute("data-i18n"); const v=t(k); if(v)e.textContent=v; });
  document.querySelectorAll("[data-i18n-ph]").forEach(e=>{ const k=e.getAttribute("data-i18n-ph"); const v=t(k); if(v)e.setAttribute("placeholder",v); });
  document.querySelectorAll("[data-i18n-title]").forEach(e=>{ const k=e.getAttribute("data-i18n-title"); const v=t(k); if(v)e.title=v; });
}

// ---------- transport ----------
const TAURI = window.__TAURI__ || null;
let MODE = "mock"; // "http" | "mock"
let TOKEN = localStorage.getItem("cortex_token") || null;

function isLocalOrigin() {
  return typeof location !== "undefined" && /^https?:$/.test(location.protocol) &&
    /^(127\.0\.0\.1|localhost|\[::1\])$/.test(location.hostname);
}
async function detectTransport() {
  // Served locally (browser `cortex serve` or the desktop app's embedded server):
  // this is ALWAYS our HTTP backend. The server may still be binding on a fresh
  // desktop launch, so wait patiently — but never silently fall back to the mock
  // sample, which would ignore the user's real data.
  if (isLocalOrigin()) {
    MODE = "http";
    for (let i = 0; i < 40; i++) { // up to ~8s
      try { const r = await fetch("/api/ping", { cache: "no-store" }); if (r.ok && (await r.json()).cortex) return; } catch (e) {}
      await new Promise(r => setTimeout(r, 200));
    }
    return; // stay "http" even if ping was slow; calls will retry
  }
  // A remote https host (e.g. the artifact preview): probe once, else demo mock.
  if (typeof location !== "undefined" && /^https?:$/.test(location.protocol)) {
    try { const r = await fetch("/api/ping", { cache: "no-store" }); if (r.ok && (await r.json()).cortex) { MODE = "http"; return; } } catch (e) {}
  }
  MODE = "mock";
}
async function api(path, { method = "GET", body = null, raw = false, auth = true } = {}) {
  if (MODE === "mock") return mockApi(path, method, body);
  const headers = {};
  if (auth && TOKEN) headers["Authorization"] = "Bearer " + TOKEN;
  if (body != null && !raw) headers["Content-Type"] = "application/json";
  const r = await fetch(path, { method, headers, body: body == null ? null : (raw ? body : JSON.stringify(body)) });
  const txt = await r.text();
  let data; try { data = txt ? JSON.parse(txt) : {}; } catch (e) { data = { raw: txt }; }
  if (!r.ok) throw new Error(data.error || r.statusText);
  return data;
}

// Run a long job (ask/run/connector_run/report_pdf) via async job + polling,
// so slow LLM calls never hit connection idle-timeouts ("failed to fetch").
async function runJob(kind, payload){
  if(MODE!=="http"){ // mock/static: call the equivalent endpoint directly
    const map={ask:"/api/ask",run:"/api/run",connector_run:"/api/connectors/run",report_pdf:"/api/report/pdf"};
    return api(map[kind]||"/api/run",{method:"POST",body:payload});
  }
  const {job_id}=await api("/api/jobs",{method:"POST",body:{kind,payload}});
  const started=Date.now();
  for(;;){
    await new Promise(r=>setTimeout(r,1300));
    let s; try{ s=await api("/api/jobs/status?id="+encodeURIComponent(job_id)); }
    catch(e){ if(Date.now()-started>600000) throw e; continue; } // tolerate a transient poll failure
    if(s.status==="done") return s.result;
    if(s.status==="error") throw new Error(s.error||"job failed");
  }
}

// ---------- state ----------
const state = {
  user: null, domains: [], dataTypes: [], provider: "auto",
  tabs: [], active: -1, notifications: [],
};
// Type palette — deliberately spread across the hue wheel so kinds don't all
// read as "another blue": teal, blue, violet, amber, magenta, green, red…
const KIND_COLOR = {
  account:"#57D7E8", person:"#63B3FF", domain:"#8B7CFF", url:"#F5B84B",
  ip:"#F59E0B", device:"#2DD4BF", wallet:"#34D399", payment:"#22C55E",
  organization:"#93C5FD", group:"#FACC15", location:"#FBBF24",
  case:"#A78BFA", report:"#C084FC", communication:"#4ADE80",
  person_alt:"#63B3FF", victim:"#F472B6", suspect:"#F87171",
  media:"#FB7185", evidence:"#FDA4AF", malware:"#EF4444",
  vulnerability:"#FB923C", incident:"#E879F9", service:"#5EEAD4",
  repository:"#A3E635", unknown:"#94A3B8"
};
const kColor = k => KIND_COLOR[k] || KIND_COLOR.unknown;
const GRAPH_BG = "#070A0F", NODE_FILL = "#151D27";
// Node sizing: bounded so nothing becomes a giant blob.
const NODE_MIN=22, NODE_MAX=44, META_MIN=40, META_MAX=88;
const nodeSize = risk => NODE_MIN + Math.sqrt(Math.max(0,Math.min(1,risk||0)))*(NODE_MAX-NODE_MIN);
// Cluster size scales with log(count) between META_MIN and META_MAX.
const metaSize = count => Math.min(META_MAX, META_MIN + Math.log2((count||2))*7);
const edgeW = conf => 0.5 + (conf||0.5)*1.3;
const KIND_SHAPE = { person:"ellipse", victim:"ellipse", suspect:"ellipse", account:"round-rectangle", device:"round-rectangle",
  ip:"diamond", url:"hexagon", domain:"hexagon", media:"round-tag", evidence:"round-tag", wallet:"pentagon", payment:"pentagon",
  group:"octagon", case:"round-rectangle", report:"round-rectangle", malware:"star", incident:"star", vulnerability:"star",
  location:"triangle", organization:"barrel", service:"round-rectangle", repository:"round-rectangle" };
const kShape = k => KIND_SHAPE[k] || "ellipse";
// Entity glyphs drawn inside each node (link-analysis style).
const ENTITY_GLYPH = {
  person:'<circle cx="12" cy="8" r="3.6"/><path d="M5 20c0-3.6 3.4-5.5 7-5.5s7 1.9 7 5.5"/>',
  victim:'<circle cx="12" cy="8" r="3.6"/><path d="M5 20c0-3.6 3.4-5.5 7-5.5s7 1.9 7 5.5"/><path d="M9 8h6"/>',
  suspect:'<circle cx="11" cy="8" r="3.6"/><path d="M4 20c0-3.6 3.4-5.5 7-5.5 1 0 2 .1 2.8.4"/><path d="M16 15l5 5M21 15l-5 5"/>',
  account:'<circle cx="12" cy="12" r="4"/><path d="M16 12v1.5a2.5 2.5 0 005 0V12a9 9 0 10-3.4 7.1"/>',
  device:'<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
  ip:'<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.6 2.4 2.6 15.6 0 18M12 3c-2.6 2.4-2.6 15.6 0 18"/>',
  domain:'<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.6 2.4 2.6 15.6 0 18M12 3c-2.6 2.4-2.6 15.6 0 18"/>',
  url:'<path d="M9 15l6-6M10.5 6.5l1-1a4 4 0 015.7 5.7l-1 1M13.5 17.5l-1 1a4 4 0 01-5.7-5.7l1-1"/>',
  media:'<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.8"/><path d="M4 18l5-4 4 3 3-2 4 3"/>',
  evidence:'<path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z"/><path d="M14 3v5h5"/>',
  communication:'<path d="M21 15a2 2 0 01-2 2H8l-4 4V5a2 2 0 012-2h13a2 2 0 012 2z"/>',
  group:'<circle cx="9" cy="9" r="2.8"/><circle cx="16" cy="10" r="2.3"/><path d="M4 19c0-2.8 2.5-4.5 5-4.5s5 1.7 5 4.5M14 19c0-2 1.3-3.2 3.5-3.2"/>',
  payment:'<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M3 10h18"/>',
  wallet:'<path d="M4 7h13a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><path d="M16 12h4M4 7l11-3v3"/>',
  location:'<path d="M12 21s7-6 7-11a7 7 0 10-14 0c0 5 7 11 7 11z"/><circle cx="12" cy="10" r="2.4"/>',
  organization:'<rect x="4" y="3" width="15" height="18" rx="1"/><path d="M8 8h2M13 8h2M8 12h2M13 12h2M10 20v-3h3v3"/>',
  malware:'<circle cx="12" cy="12" r="3.6"/><path d="M12 8.4V4M8.4 12H4M15.6 12H20M9.4 9.4L6.5 6.5M14.6 9.4l2.9-2.9M9.4 14.6l-2.9 2.9M14.6 14.6l2.9 2.9"/>',
  incident:'<path d="M10.3 4L3 17.5A2 2 0 004.7 20.5h14.6A2 2 0 0021 17.5L13.7 4a2 2 0 00-3.4 0z"/><path d="M12 9.5v4M12 16.5h.01"/>',
  vulnerability:'<path d="M12 3l8 4v5c0 5-4 8-8 9-4-1-8-4-8-9V7z"/><path d="M12 9v3.5M12 15.5h.01"/>',
  case:'<path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z"/><path d="M14 3v5h5"/>',
  report:'<path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z"/><path d="M14 3v5h5M9 13h6M9 16h4"/>',
  service:'<path d="M18 10a4 4 0 00-7.7-1.4A3.5 3.5 0 108.5 16H18a3 3 0 000-6z"/>',
  repository:'<circle cx="7" cy="6" r="2.2"/><circle cx="7" cy="18" r="2.2"/><circle cx="17" cy="8" r="2.2"/><path d="M7 8.2v7.6M17 10.2c0 3.5-4.5 2.8-6.5 4.3"/>',
  unknown:'<circle cx="12" cy="12" r="9"/><path d="M9.5 9.2a2.6 2.6 0 013.7 2.1c0 1.6-2.2 2-2.2 3.2M12 17.2h.01"/>',
};
function nodeIcon(kind){ const p=ENTITY_GLYPH[kind]||ENTITY_GLYPH.unknown;
  const s=`<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#eef4fa' stroke-width='1.9' stroke-linecap='round' stroke-linejoin='round'>${p}</svg>`;
  return "data:image/svg+xml;utf8,"+encodeURIComponent(s); }
const bandOf = s => s>=0.85?"critical":s>=0.6?"high":s>=0.35?"medium":"low";
const bandColor = b => ({low:"#34d399",medium:"#f59e0b",high:"#fb7185",critical:"#ef4444"}[b]||"#34d399");

// ---------- dom helpers ----------
const $ = s => document.querySelector(s);
const $$ = s => Array.from(document.querySelectorAll(s));
const el = (t,c,txt) => { const e=document.createElement(t); if(c)e.className=c; if(txt!=null)e.textContent=txt; return e; };
const esc = s => String(s).replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
function toast(m,k){ const t=$("#toast"); t.textContent=m; t.className="toast "+(k||""); t.hidden=false; clearTimeout(t._t); t._t=setTimeout(()=>t.hidden=true,3200); }
function setSync(k,l){ const d=$("#syncStatus .dot"); d.className="dot "+k; $("#syncLabel").textContent=l; }
const activeTab = () => state.tabs[state.active] || null;

// ---------- lucide-style icons (inline SVG) ----------
const ICONS = {
  dashboard:'<rect x="3" y="3" width="7" height="9" rx="1"/><rect x="14" y="3" width="7" height="5" rx="1"/><rect x="14" y="12" width="7" height="9" rx="1"/><rect x="3" y="16" width="7" height="5" rx="1"/>',
  graph:'<circle cx="5" cy="6" r="2.2"/><circle cx="19" cy="7" r="2.2"/><circle cx="12" cy="17" r="2.2"/><path d="M7 7l3.5 8M17 8l-3.5 7"/>',
  intel:'<path d="M12 3l1.9 4.6L18 9l-4.1 1.4L12 15l-1.9-4.6L6 9l4.1-1.4z"/><path d="M18 15l.8 2 .2.8"/><path d="M5 16l.6 1.6"/>',
  entities:'<path d="M12 2l8 4.5v9L12 20l-8-4.5v-9z"/><path d="M12 2v18M4 6.5l8 4.5 8-4.5"/>',
  sources:'<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/>',
  timeline:'<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  alerts:'<path d="M10.3 3.8L2 18a2 2 0 001.7 3h16.6A2 2 0 0022 18L13.7 3.8a2 2 0 00-3.4 0z"/><path d="M12 9v4M12 17h.01"/>',
  reports:'<path d="M14 3H6a2 2 0 00-2 2v14a2 2 0 002 2h12a2 2 0 002-2V9z"/><path d="M14 3v6h6M8 13h8M8 17h5"/>',
  operations:'<path d="M12 15a3 3 0 100-6 3 3 0 000 6z"/><path d="M19 12a7 7 0 00-.1-1l2-1.6-2-3.4-2.4 1a7 7 0 00-1.7-1l-.4-2.5H10l-.4 2.5a7 7 0 00-1.7 1l-2.4-1-2 3.4 2 1.6a7 7 0 000 2l-2 1.6 2 3.4 2.4-1a7 7 0 001.7 1l.4 2.5h4l.4-2.5a7 7 0 001.7-1l2.4 1 2-3.4-2-1.6a7 7 0 00.1-1z"/>',
  settings:'<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 00-.1-1l2-1.6-2-3.4-2.4 1a7 7 0 00-1.7-1l-.4-2.5H10l-.4 2.5a7 7 0 00-1.7 1l-2.4-1-2 3.4 2 1.6a7 7 0 000 2l-2 1.6 2 3.4 2.4-1a7 7 0 001.7 1l.4 2.5h4l.4-2.5a7 7 0 001.7-1l2.4 1 2-3.4-2-1.6a7 7 0 00.1-1z"/>',
  plus:'<path d="M12 5v14M5 12h14"/>', search:'<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/>',
  play:'<path d="M6 4l14 8-14 8z"/>', spark:'<path d="M12 3l1.9 4.6L18 9l-4.1 1.4L12 15l-1.9-4.6L6 9l4.1-1.4z"/>',
  bell:'<path d="M18 8a6 6 0 00-12 0c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 01-3.4 0"/>',
  logo:'<path d="M12 2l8 4.5v9L12 20l-8-4.5v-9z"/><circle cx="12" cy="11" r="3"/>',
  close:'<path d="M18 6L6 18M6 6l12 12"/>', plusc:'<circle cx="12" cy="12" r="9"/><path d="M12 8v8M8 12h8"/>',
  zoomin:'<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3M11 8v6M8 11h6"/>', zoomout:'<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3M8 11h6"/>', fit:'<path d="M4 8V4h4M20 8V4h-4M4 16v4h4M20 16v4h-4"/>',
  link:'<path d="M9 15l6-6M10 6l1-1a4 4 0 015.7 5.7l-1 1M14 18l-1 1a4 4 0 01-5.7-5.7l1-1"/>', trash:'<path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14"/>', run:'<path d="M6 4l14 8-14 8z"/>',
};
function svg(name){ return `<svg class="ic" viewBox="0 0 24 24">${ICONS[name]||''}</svg>`; }
const NAVICON={dashboard:'dashboard',graph:'graph',intelligence:'intel',entities:'entities',sources:'sources',timeline:'timeline',alerts:'alerts',reports:'reports',operations:'operations',settings:'settings'};
function applyIcons(){
  $$('.nav li').forEach(li=>{ const ic=NAVICON[li.dataset.view]; const ni=li.querySelector('.ni'); if(ic&&ni) ni.innerHTML=svg(ic); });
  $$('.logo').forEach(l=>l.innerHTML=svg('logo'));
  const set=(sel,name,keepText)=>{ const e=$(sel); if(e){ e.innerHTML=svg(name)+(keepText?(' '+keepText):''); } };
  set('#btnNewProject','plus'); const gs=$('.gs-icon'); if(gs)gs.innerHTML=svg('search');
  set('#btnRun','play',t('btn.run')); set('#btnAsk','spark',t('btn.askai'));
  const bn=$('#btnNotifications'); if(bn){ bn.innerHTML=svg('bell')+'<span class="badge" id="notifBadge" hidden>0</span>'; }
  const sl=$('.splash-logo'); if(sl)sl.innerHTML=`<svg class="ic" style="width:52px;height:52px" viewBox="0 0 24 24">${ICONS.logo}</svg>`;
  const ge=$('.ge-icon'); if(ge)ge.innerHTML=`<svg class="ic" style="width:44px;height:44px" viewBox="0 0 24 24">${ICONS.graph}</svg>`;
}

// ---------- boot ----------
async function boot() {
  applyIcons();
  document.documentElement.lang=LANG; applyI18n();
  await detectTransport();
  const steps = [];
  const push = (label, ok, cls) => steps.push({label, ok, cls});
  let health = null;
  try { health = await api("/api/health", { auth: false }); } catch (e) {}
  const bootList = $("#bootList"), bar = $("#bootBar");
  const modules = (health && health.modules) || ["ingestion","normalization","entity-extraction","graph-correlation","risk-prioritization","investigation","audit","connectors","ai-copilot"];
  const lines = [];
  modules.forEach(m => lines.push({t:`module ${m}`, ok:true}));
  const backends = (health && health.backends) || [];
  backends.forEach(b => lines.push({t:`backend ${b.name}`, ok:b.ok, detail:b.detail}));
  const plugins = (health && health.plugins) || [];
  lines.push({t:`plugins loaded: ${plugins.length}`, ok:true});
  lines.push({t:`transport: ${MODE}`, ok: MODE==="http"});

  for (let i=0;i<lines.length;i++) {
    const ln = lines[i];
    const row = el("div","bi");
    row.style.animationDelay = (i*45)+"ms";
    row.innerHTML = `<span class="${ln.ok?'ok':'warn'}">${ln.ok?'✓':'!'}</span> ${esc(ln.t)}`;
    bootList.appendChild(row);
    bar.style.width = Math.round(((i+1)/lines.length)*100)+"%";
  }
  await new Promise(r=>setTimeout(r, Math.min(1400, 250+lines.length*55)));

  // Static/artifact preview: skip auth, go straight into the demo.
  if (MODE === "mock") { state.user = { display_name: "Demo Analyst", email: "demo@cortex.local", role: "admin" }; TOKEN = "mock"; $("#splash").hidden = true; enterApp(); return; }

  // decide auth vs app
  let status = { has_accounts: true };
  try { status = await api("/api/auth/status", { auth: false }); } catch (e) {}
  if (TOKEN) {
    try { state.user = await api("/api/me"); $("#splash").hidden=true; enterApp(); return; } catch (e) { TOKEN=null; localStorage.removeItem("cortex_token"); }
  }
  $("#splash").hidden = true;
  showAuth(status.has_accounts ? "login" : "register");
}

// ---------- auth ----------
let authMode = "login";
function showAuth(mode) {
  authMode = mode;
  $("#auth").hidden = false; $("#app").hidden = true;
  $$(".auth-tab").forEach(t=>t.classList.toggle("active", t.dataset.auth===mode));
  $("#nameField").hidden = mode!=="register";
  $("#pwHint").hidden = mode!=="register";
  $("#authSub").textContent = mode==="register" ? "Create your workspace account" : "Sign in to your workspace";
  $("#authSubmit").textContent = mode==="register" ? "Create account" : "Sign in";
  $("#authError").textContent = "";
}
$$(".auth-tab").forEach(t=>t.addEventListener("click",()=>showAuth(t.dataset.auth)));
$("#authForm").addEventListener("submit", async e => {
  e.preventDefault();
  const email=$("#aEmail").value.trim(), password=$("#aPassword").value, name=$("#aName").value.trim();
  $("#authError").textContent = "";
  try {
    const res = authMode==="register"
      ? await api("/api/auth/register",{method:"POST",auth:false,body:{email,display_name:name,password}})
      : await api("/api/auth/login",{method:"POST",auth:false,body:{email,password}});
    TOKEN = res.token; localStorage.setItem("cortex_token", TOKEN); state.user = res.user;
    $("#auth").hidden = true; enterApp();
  } catch (err) { $("#authError").textContent = err.message || String(err); }
});

async function logout() {
  try { await api("/api/auth/logout",{method:"POST"}); } catch(e){}
  TOKEN=null; localStorage.removeItem("cortex_token"); state.user=null;
  location.reload();
}

// ---------- app enter ----------
async function enterApp() {
  $("#app").hidden = false;
  applyIcons();
  try { state.domains = await api("/api/domains"); } catch(e){ state.domains=[]; }
  try { state.dataTypes = await api("/api/data_types"); } catch(e){ state.dataTypes=[]; }
  $("#avatar").textContent = (state.user?.display_name||state.user?.email||"OP").slice(0,2).toUpperCase();
  buildProviderSelect();
  refreshDoctor(); renderConnectorCards(); renderPluginExample();
  $("#providerPill").textContent = "provider: "+state.provider;
  // Onboarding: ask country (BR/US) once, THEN show the project launcher.
  try { const cfg=await api("/api/config"); state.country=cfg.country; if(!cfg.onboarded){ onboardCountry(()=>showLauncher()); return; } } catch(e){}
  showLauncher();
}

// Project launcher — the entry screen: pick a saved/recent project or create new.
async function showLauncher(){
  $("#app").hidden=true; $("#launcher").hidden=false;
  applyI18n();
  const grid=$("#launcherGrid"); grid.innerHTML='<div class="empty">Loading…</div>';
  let list=[]; try{ list=await api("/api/projects"); }catch(e){}
  grid.innerHTML="";
  if(!list.length){ grid.innerHTML=`<div class="empty">${esc(t("launcher.empty"))}</div>`; return; }
  list.forEach(p=>{ const c=el("div","proj-card");
    const when=p.updated_at?new Date(p.updated_at*1000).toISOString().slice(0,10):"";
    c.innerHTML=`<div class="pc-name">${esc(p.name)}</div>
      <div class="pc-meta"><span class="pc-vert">${esc(p.domain)}</span><span>${p.activity_count} activities</span><span>${p.connector_count} sources</span>${p.has_result?'<span>✓ analyzed</span>':''}</div>
      <div class="pc-foot"><span class="muted" style="margin:0">updated ${when}</span><span class="pc-x" title="Delete">✕</span></div>`;
    c.addEventListener("click",e=>{ if(e.target.classList.contains("pc-x")) return; enterWorkspace(p.id); });
    c.querySelector(".pc-x").addEventListener("click",async e=>{ e.stopPropagation(); if(!confirm(`Delete project "${p.name}"?`))return; try{ await api("/api/projects/delete",{method:"POST",body:{id:p.id}}); showLauncher(); }catch(err){toast(err.message,"err");} });
    grid.appendChild(c); });
}
// Enter the main workspace with a project open.
async function enterWorkspace(projectId){ $("#launcher").hidden=true; $("#app").hidden=false; applyIcons();
  await loadProjects(); if(projectId) await openProject(projectId); else showView("dashboard"); }
$("#lnNew")&&$("#lnNew").addEventListener("click",()=>{ $("#launcher").hidden=true; $("#app").hidden=false; applyIcons(); newProjectModal(); });
$("#lnImport")&&$("#lnImport").addEventListener("click",()=>importProjectFlow(()=>{}));
// Header project actions
$("#btnHdrOpen")&&$("#btnHdrOpen").addEventListener("click",showLauncher);
$("#btnHdrImport")&&$("#btnHdrImport").addEventListener("click",()=>importProjectFlow());
$("#btnHdrExport")&&$("#btnHdrExport").addEventListener("click",()=>exportActiveProject());
function importProjectFlow(){ pickFile(async text=>{ try{ const p=await api("/api/projects/import",{method:"POST",raw:true,body:text}); toast("Project imported","ok"); $("#launcher").hidden=true; $("#app").hidden=false; applyIcons(); await loadProjects(); openProject(p.id); }catch(e){toast("Import failed: "+e.message,"err");} }); }
async function exportActiveProject(){ const t=activeTab(); if(!t){toast("Open a project first","err");return;}
  try{ const bundle = MODE==="mock" ? JSON.stringify(t.project,null,2) : await (await fetch(`/api/projects/export?id=${encodeURIComponent(t.project.id)}`,{headers:{Authorization:"Bearer "+TOKEN}})).text();
    downloadText(`${t.project.name.replace(/\s+/g,"_")}.cortex`, bundle); toast("Project exported","ok"); }catch(e){toast(e.message,"err");} }

// ---------- projects & tabs ----------
async function loadProjects() {
  const list = await api("/api/projects").catch(()=>[]);
  const wrap = $("#projectList"); wrap.innerHTML = "";
  if (!list.length) wrap.innerHTML = '<div class="empty">No projects yet — create one.</div>';
  list.forEach(p => {
    const li = el("div","li");
    const l = el("div","l"); l.appendChild(el("span","label",`${p.name}`));
    li.appendChild(l);
    const meta = el("span","chip",`${p.domain} · ${p.activity_count} acts`);
    li.appendChild(meta);
    li.addEventListener("click",()=>openProject(p.id));
    wrap.appendChild(li);
  });
}

async function openProject(id) {
  const existing = state.tabs.findIndex(t=>t.project.id===id);
  if (existing>=0) { switchTab(existing); return; }
  let project; try { project = await api(`/api/projects/get?id=${encodeURIComponent(id)}`); } catch(e){ toast("Cannot open project: "+e.message,"err"); return; }
  const graph = project.last_result ? consolidatedToGraph(project.last_result) : {nodes:[],edges:[]};
  state.tabs.push({ project, graph, result: project.last_result||null });
  switchTab(state.tabs.length-1);
  renderTabs();
}

function switchTab(i) {
  state.active = i; renderTabs();
  const t = activeTab(); if (!t) return;
  const d = state.domains.find(x=>x.slug===t.project.domain);
  $("#verticalPill").textContent = t.project.domain;
  renderAll();
  showView(currentView);
}

function closeTab(i, ev) {
  ev && ev.stopPropagation();
  state.tabs.splice(i,1);
  if (state.active>=state.tabs.length) state.active=state.tabs.length-1;
  renderTabs();
  if (state.active>=0) switchTab(state.active); else { clearGraph(); renderAll(); showView("dashboard"); }
}

function renderTabs() {
  const wrap = $("#tabs"); wrap.innerHTML = "";
  state.tabs.forEach((t,i)=>{
    const tab = el("div","tab"+(i===state.active?" active":""));
    tab.appendChild(el("span",null,t.project.name));
    const x = el("span","tclose","✕"); x.addEventListener("click",e=>closeTab(i,e));
    tab.appendChild(x);
    tab.addEventListener("click",()=>switchTab(i));
    wrap.appendChild(tab);
  });
}

let onboardSel="BR";
function onboardCountry(done){
  openModal("Welcome — set your region", `
    <p class="muted">CortexIntel tailors identity/KYC checks and disclaimers to your country. Supported now: Brazil & United States.</p>
    <div class="country-grid">
      <div class="country-opt sel" data-c="BR"><div class="flag">🇧🇷</div><div class="cn">Brazil</div><div class="muted" style="margin:0">CPF · LGPD</div></div>
      <div class="country-opt" data-c="US"><div class="flag">🇺🇸</div><div class="cn">United States</div><div class="muted" style="margin:0">SSN · privacy</div></div>
    </div>
    <div class="disclaimer">Person/identity data is regulated. Processing requires a lawful basis under LGPD (BR) / GDPR & state law (US). Validation is decision-support, never a definitive identity ruling.</div>
  `,[{label:"Continue",cls:"primary",act:async()=>{ try{ await api("/api/config",{method:"POST",body:{country:onboardSel,onboarded:true}}); state.country=onboardSel; }catch(e){} closeModal(); toast("Region set: "+onboardSel,"ok"); if(typeof done==="function") done(); }}]);
  onboardSel="BR";
  setTimeout(()=>{ $$(".country-opt").forEach(o=>o.addEventListener("click",()=>{ $$(".country-opt").forEach(x=>x.classList.remove("sel")); o.classList.add("sel"); onboardSel=o.dataset.c; })); },40);
}
function newProjectModal() {
  const domainOpts = state.domains.map(d=>`<option value="${d.slug}">${esc(d.title)}</option>`).join("");
  openModal("New project", `
    <div class="field">Project name<input id="npName" placeholder="e.g. Case 2026-114 · Q3 review · Threat sweep" /></div>
    <div class="field">Business vertical<select id="npDomain" class="select">${domainOpts}</select></div>
    <div class="field">Description<textarea id="npDesc" rows="2" placeholder="what this project investigates"></textarea></div>
    <div class="field">Data context for the AI — describe what the data IS<textarea id="npAI" rows="4" placeholder="The more you tell the AI, the sharper the analysis. e.g.:
• What each dataset is (customer export, transaction log, hotline tips…)
• What the columns mean, especially non-obvious ones
• What you're trying to find or decide
• What counts as risk/success here; what to ignore (test accounts, internal traffic…)"></textarea></div>
    <div class="field">Import a file now (optional)<div style="display:flex;gap:8px"><input id="npFile" placeholder="no file selected" readonly style="flex:1" /><button class="btn ghost" id="npBrowse">Browse…</button></div></div>
    <div class="modal-note" id="npNote"></div>
  `, [
    {label:"Cancel", cls:"ghost", act:closeModal},
    {label:"Create", cls:"primary", act: async ()=>{
      const name=$("#npName").value.trim(); if(!name){ toast("Name required","err"); return; }
      try {
        const p = await api("/api/projects",{method:"POST",body:{name,domain:$("#npDomain").value,description:$("#npDesc").value,ai_instructions:$("#npAI").value}});
        closeModal(); await loadProjects(); await openProject(p.id); pushNotif("project",`Project "${p.name}" created`);
        if(npUploadPath){ const tb=activeTab(); if(tb){ setSync("busy","running"); try{ const result=await runJob("run",{inputs:[npUploadPath],domain:p.domain,provider:state.provider,maxRecords:4000,projectId:p.id}); tb.result=result; tb.graph=consolidatedToGraph(result); tb.project=await api(`/api/projects/get?id=${encodeURIComponent(p.id)}`).catch(()=>tb.project); setSync("ok","complete"); renderAll(); showView("graph"); setTimeout(()=>{initCy();if(cy)cy.fit(cy.elements(),50);},700);}catch(e){setSync("err","failed");toast(e.message,"err");} } }
      } catch(e){ toast(e.message,"err"); }
    }}
  ]);
  npUploadPath=null;
  setTimeout(()=>{ const b=$("#npBrowse"); if(b) b.addEventListener("click",()=>pickServerPath(path=>{ npUploadPath=path; $("#npFile").value=path.split("/").pop(); }, {title:"Choose a file or a folder of media", folders:true, accept:".csv,.tsv,.json,.jsonl,.ndjson,.png,.jpg,.jpeg,.gif,.webp,.mp4,.mov,.mp3,.wav,.pdf"})); },40);
  setTimeout(()=>$("#npName")&&$("#npName").focus(),50);
}

// ---------- graph data ----------
function consolidatedToGraph(c) {
  let nodes=[];
  Object.values(c.entities||{}).forEach(a=>{ if(Array.isArray(a)) nodes=nodes.concat(a); });
  nodes = nodes.map(n=>({ id:n.id, kind:n.kind, label:n.label, risk:n.risk_score||0, band:n.risk_band||bandOf(n.risk_score||0),
    attributes:n.attributes||{}, tags:n.tags||[], sources:n.sources||[], sensitive:!!n.sensitive }));
  const edges=(c.relationships||[]).map(r=>({source:r.source_id,target:r.target_id,type:r.rel_type,conf:r.confidence||0.5}));
  return {nodes, edges, meta:{risk:c.ai_assessments,investigation:c.investigation,governance:c.governance,audit:c.audit_events||[],assessment:c.assessment||[],nba:c.next_best_actions||[]}};
}

// ---------- cytoscape ----------
let cy = null;
function initCy() {
  if (cy) return cy;
  try { if (window.cytoscapeFcose) cytoscape.use(window.cytoscapeFcose); } catch(e){}
  cy = cytoscape({
    container: $("#cy"),
    wheelSensitivity: 0.25,
    // Performance: keep large graphs smooth and prevent WebView lockups.
    hideEdgesOnViewport: true,
    textureOnViewport: true,
    motionBlur: false,
    pixelRatio: 1,
    style: [
      { selector:"node", style:{
        "background-color": NODE_FILL, "background-image":"data(icon)", "background-width":"52%", "background-height":"52%", "background-fit":"none", "background-clip":"none",
        "width":"data(size)", "height":"data(size)", "shape":"ellipse",
        "label":"data(label)", "font-size":"9px", "font-weight":600, "font-family":"SF Mono, Menlo, monospace", "color":"#E6EDF7",
        "text-wrap":"wrap", "text-max-width":"88px", "text-valign":"bottom", "text-margin-y":5, "min-zoomed-font-size":8,
        "text-outline-color":"#070A0F", "text-outline-width":2, "text-outline-opacity":0.85,
        "border-width":"data(bw)", "border-color":"data(kc)", "border-opacity":0.95,
        "transition-property":"opacity border-width", "transition-duration":"140ms" }},
      { selector:"node[halo]", style:{ "underlay-color":"data(hc)", "underlay-padding":6, "underlay-opacity":0.4 }},
      // perf mode: solid coloured dot, no SVG icon
      { selector:"node.plain", style:{ "background-image":"none", "background-color":"data(kc)" }},
      // ---- edges: discreet by default ----
      { selector:"edge", style:{
        "width":"data(w)", "line-color":"rgba(148,163,184,0.16)", "target-arrow-color":"rgba(148,163,184,0.24)",
        "target-arrow-shape":"triangle", "arrow-scale":0.55, "curve-style":"bezier",
        "label":"", "font-size":"7px", "font-family":"SF Mono, Menlo, monospace", "color":"rgba(200,214,230,0.7)",
        "text-rotation":"autorotate", "text-background-color":"#070A0F", "text-background-opacity":0.7, "text-background-padding":2,
        "transition-property":"opacity line-color width", "transition-duration":"140ms" }},
      // ---- focus / hover / selection ----
      { selector:"node.focused", style:{ "border-width":3.5, "border-color":"#E6EDF7", "underlay-color":"data(kc)", "underlay-padding":10, "underlay-opacity":0.55, "z-index":50 }},
      { selector:"node:selected", style:{ "border-width":3.5, "border-color":"#E6EDF7", "underlay-color":"data(kc)", "underlay-padding":10, "underlay-opacity":0.55, "z-index":50 }},
      { selector:"node.neighbor", style:{ "border-opacity":1, "z-index":40 }},
      { selector:"edge.connected", style:{ "line-color":"data(kc)", "target-arrow-color":"data(kc)", "width":"mapData(w, 0, 3, 1.4, 3)", "opacity":1, "label":"data(type)", "min-zoomed-font-size":9, "z-index":40 }},
      { selector:"node.dim", style:{ "opacity":0.12 }},
      { selector:"edge.dim", style:{ "opacity":0.05 }},
      { selector:".faded", style:{ "opacity":0.1 }},
      { selector:".hyp", style:{ "line-style":"dashed", "line-color":"#8B7CFF", "border-color":"#8B7CFF", "border-style":"dashed" }},
      { selector:".fresh", style:{ "underlay-color":"#34D399", "underlay-padding":10, "underlay-opacity":0.55 }},
      { selector:"node.pathhl", style:{ "border-width":3, "border-color":"#57D7E8", "underlay-color":"#57D7E8", "underlay-padding":8, "underlay-opacity":0.5, "opacity":1, "z-index":60 }},
      { selector:"edge.pathhl", style:{ "line-color":"#57D7E8", "target-arrow-color":"#57D7E8", "width":3, "opacity":1, "label":"data(type)", "z-index":60 }},
      // ---- meta cluster nodes: two-line label ----
      { selector:"node.metanode", style:{ "shape":"round-hexagon", "background-color":"#12202B", "border-color":"data(kc)", "border-width":2.5,
        "background-image":"data(icon)", "background-width":"40%", "background-height":"40%",
        "label":"data(label)", "text-wrap":"wrap", "font-size":"11px", "color":"#E6EDF7", "text-valign":"bottom", "text-margin-y":6,
        "text-outline-color":"#070A0F", "text-outline-width":2 }},
      // show a global "zoomed-in" edge label only when very close
      { selector:"core", style:{} },
    ],
  });
  cy.on("tap","node", ev=>{ const id=ev.target.id(); if(linkMode){ finishLink(id); return; } if(pathSource){ finishPath(id); return; } selectNode(id); });
  cy.on("dbltap","node", ev=>{ const id=ev.target.id(); const t=activeTab(); if(!t)return; if(t._metas&&t._metas[id]) expandCluster(id); else if((t.clusterMode||"none")!=="none") collapseNodeCluster(id); });
  cy.on("tap", ev=>{ if(ev.target===cy){ clearFocus(); cy.$(":selected").unselect(); $("#context").hidden=true; } });
  cy.on("cxttap","node", ev=>{ const e=ev.originalEvent; if(linkMode) finishLink(ev.target.id()); else openCtxMenu(e.clientX,e.clientY,ev.target.id()); });
  // Hover: transient focus on node + neighbors + connected edges, dim the rest.
  cy.on("mouseover","node", ev=>{ if(!cyPinned) focusNeighborhood(ev.target, false); });
  cy.on("mouseout","node", ev=>{ if(!cyPinned) clearFocus(); });
  cy.on("pan zoom", ()=>scheduleMinimap());
  cy.on("layoutstop render", ()=>scheduleMinimap());
  // Responsiveness: keep the canvas sized to its container as the window/panels
  // resize (dragging the window edge, opening/closing the dossier, etc.).
  let _rzT; const onResize=()=>{ clearTimeout(_rzT); _rzT=setTimeout(()=>{ if(cy){ cy.resize(); scheduleMinimap(); } },120); };
  window.addEventListener("resize", onResize);
  if(window.ResizeObserver){ try{ new ResizeObserver(onResize).observe($("#cy")); }catch(e){} }
  return cy;
}
// Focus a node + its neighbors; dim everything else. `pin` fixes it until cleared.
let cyPinned=false;
function focusNeighborhood(node, pin){ if(!cy||!node||!node.length)return; if(pin!==undefined)cyPinned=pin;
  const nb=node.closedNeighborhood(); const others=cy.elements().difference(nb);
  cy.batch(()=>{ others.addClass("dim"); nb.removeClass("dim");
    node.addClass("focused"); node.neighborhood("node").addClass("neighbor"); node.connectedEdges().addClass("connected"); }); }
function clearFocus(){ if(!cy)return; cyPinned=false; cy.batch(()=>{ cy.elements().removeClass("dim focused neighbor connected faded pathhl"); }); }

// ----- clustering (collapse/expand) -----
function unionComponents(g){ const parent={}; g.nodes.forEach(n=>parent[n.id]=n.id);
  const find=x=>{ while(parent[x]!==x){ parent[x]=parent[parent[x]]; x=parent[x]; } return x; };
  g.edges.forEach(e=>{ if(parent[e.source]!=null&&parent[e.target]!=null){ const a=find(e.source),b=find(e.target); if(a!==b)parent[a]=b; } });
  const idx={}; let k=0; const comp={}; g.nodes.forEach(n=>{ const r=find(n.id); if(idx[r]==null)idx[r]=k++; comp[n.id]=idx[r]; }); return comp;
}
function clusterKey(n, mode, comp){ return mode==="kind" ? "k:"+n.kind : "c:"+(comp[n.id]!=null?comp[n.id]:("s"+n.id)); }
const fmtNum = n => (n||0).toLocaleString("en-US");
// Two-line label: "account\n1,999"
function metaLabel(cid, count){ return (cid.startsWith("k:")?cid.slice(2):"cluster")+"\n"+fmtNum(count); }
// Build the render model applying cluster collapse; also returns metas map.
function computeRenderModel(t){ const g=t.graph; const mode=t.clusterMode||"none";
  t._metas={};
  if(mode==="none") return {nodes:g.nodes, edges:g.edges};
  const comp = mode==="component" ? unionComponents(g) : {};
  const members={}; g.nodes.forEach(n=>{ const c=clusterKey(n,mode,comp); (members[c]=members[c]||[]).push(n); });
  const collapsed=t.collapsed||new Set();
  const toRender={}; const outNodes=[];
  const caps=t._expandCap||{};
  Object.entries(members).forEach(([cid,list])=>{
    const domOf=arr=>{ const k={}; arr.forEach(n=>k[n.kind]=(k[n.kind]||0)+1); return Object.entries(k).sort((a,b)=>b[1]-a[1])[0][0]; };
    if(collapsed.has(cid) && list.length>1){ const dom=domOf(list); const maxRisk=Math.max(...list.map(n=>n.risk||0),0);
      t._metas[cid]={ id:cid, label:metaLabel(cid,list.length), count:list.length, kind:dom, risk:maxRisk, members:list.map(n=>n.id) };
      list.forEach(n=>toRender[n.id]=cid);
      outNodes.push({ meta:true, id:cid, label:metaLabel(cid,list.length), kind:dom, risk:maxRisk, count:list.length });
    } else if(caps[cid] && caps[cid].size < list.length){ // partially expanded: top-N individual + residual meta
      const cap=caps[cid]; const shown=list.filter(n=>cap.has(n.id)); const rest=list.filter(n=>!cap.has(n.id));
      shown.forEach(n=>{ toRender[n.id]=n.id; outNodes.push(n); });
      const rid=cid+":rest"; const dom=domOf(rest); const maxRisk=Math.max(...rest.map(n=>n.risk||0),0);
      t._metas[rid]={ id:rid, label:metaLabel(cid,rest.length), count:rest.length, kind:dom, risk:maxRisk, members:rest.map(n=>n.id) };
      rest.forEach(n=>toRender[n.id]=rid);
      outNodes.push({ meta:true, id:rid, label:"+ "+fmtNum(rest.length)+"\nmore", kind:dom, risk:maxRisk, count:rest.length });
    } else { list.forEach(n=>{ toRender[n.id]=n.id; outNodes.push(n); }); }
  });
  const seen={}; const outEdges=[];
  g.edges.forEach(e=>{ const s=toRender[e.source], tt=toRender[e.target]; if(s==null||tt==null||s===tt) return; const key=s+"|"+tt;
    if(seen[key]){ seen[key].conf=Math.max(seen[key].conf,e.conf||0.5); return; } const ed={source:s,target:tt,type:e.type,conf:e.conf||0.5}; seen[key]=ed; outEdges.push(ed); });
  return {nodes:outNodes, edges:outEdges};
}
function setClusterMode(mode){ const t=activeTab(); if(!t)return; t.clusterMode=mode;
  if(mode==="none"){ t.collapsed=new Set(); } else { // collapse all clusters by default
    const g=t.graph; const comp=mode==="component"?unionComponents(g):{}; const set=new Set(); g.nodes.forEach(n=>set.add(clusterKey(n,mode,comp))); t.collapsed=set; }
  renderGraph(); }
function expandCluster(cid){ const t=activeTab(); if(!t||!t.collapsed)return;
  const meta=t._metas&&t._metas[cid]; const size=meta?meta.count:0;
  // Expanding a huge cluster (e.g. 1,999 accounts) would dump thousands of nodes
  // and freeze the WebView. Offer a bounded view instead of blowing up.
  if(size>600){
    openModal(`Expand ${meta.label.replace("\n"," · ")}?`, `<p class="muted">This cluster has <b>${fmtNum(size)}</b> members. Rendering all at once can be heavy — choose how much to show.</p>
      <div class="field">How to expand<select id="expMode" class="select">
        <option value="risk">Top 300 by risk (recommended)</option>
        <option value="custom">Custom amount by risk…</option>
        <option value="prompt">By instruction (AI)…</option>
        <option value="all">All ${fmtNum(size)} (may be slow)</option>
      </select></div>
      <div class="field" id="expCustomWrap" hidden>Amount to show<input id="expCount" type="number" min="1" max="${size}" value="300" /></div>
      <div class="field" id="expPromptWrap" hidden>Instruction<input id="expPrompt" placeholder='e.g. "only suspended accounts", "high-risk in Finance", "flagged transfers"' /><div class="modal-note">The instruction filters this cluster's members by their labels, tags and attributes.</div></div>`,
      [{label:"Cancel",cls:"ghost",act:closeModal},{label:"Expand",cls:"primary",act:()=>{
        const mode=$("#expMode").value;
        if(mode==="all"){ closeModal(); doExpand(cid, null); }
        else if(mode==="risk"){ closeModal(); doExpand(cid, 300); }
        else if(mode==="custom"){ let c=parseInt($("#expCount").value,10); if(!(c>0)) c=300; closeModal(); doExpand(cid, Math.min(c,size)); }
        else if(mode==="prompt"){ const q=$("#expPrompt").value.trim(); if(!q){ toast("Enter an instruction","err"); return; } closeModal(); doExpandByPrompt(cid, q); }
      }}]);
    // toggle the custom / prompt inputs by selected mode
    setTimeout(()=>{ const sel=$("#expMode"); if(!sel)return; const sync=()=>{ $("#expCustomWrap").hidden=sel.value!=="custom"; $("#expPromptWrap").hidden=sel.value!=="prompt"; }; sel.addEventListener("change",sync); sync(); },40);
    return;
  }
  doExpand(cid, null);
}
// Expand a cluster by a natural-language instruction: keep only members whose
// label / tags / attributes match the query terms. Deterministic (offline) — no
// invented members, only members already in the cluster that match the filter.
function doExpandByPrompt(cid, q){
  const t=activeTab(); if(!t)return;
  const meta=t._metas&&t._metas[cid]; if(!meta)return;
  const members=new Set(meta.members);
  const terms=q.toLowerCase().split(/[\s,]+/).filter(w=>w.length>1);
  const hay=n=>[n.label,(n.tags||[]).join(" "),Object.entries(n.attributes||{}).map(([k,v])=>k+" "+v).join(" ")].join(" ").toLowerCase();
  const matched=t.graph.nodes.filter(n=>members.has(n.id)).filter(n=>{ const h=hay(n); return terms.every(term=>h.includes(term)); });
  if(!matched.length){ toast(`No members match "${q}" — showing top 300 by risk instead`,"err"); doExpand(cid, 300); return; }
  t.collapsed.delete(cid);
  t._expandCap=t._expandCap||{}; t._expandCap[cid]=new Set(matched.map(n=>n.id));
  renderGraph();
  toast(`Expanded ${matched.length} member(s) matching "${q}"`);
}
function doExpand(cid, cap){ const t=activeTab(); if(!t)return; t.collapsed.delete(cid);
  if(cap){ // keep only the top-`cap` members of this cluster visible; re-collapse the rest into a residual meta
    const meta=t._metas[cid]; const members=new Set(meta.members);
    const ranked=t.graph.nodes.filter(n=>members.has(n.id)).sort((a,b)=>(b.risk||0)-(a.risk||0));
    t._expandCap=t._expandCap||{}; t._expandCap[cid]=new Set(ranked.slice(0,cap).map(n=>n.id));
  } else if(t._expandCap){ delete t._expandCap[cid]; }
  renderGraph();
  toast(cap?`Expanded top ${cap} of cluster`:"Cluster expanded");
}
function collapseNodeCluster(id){ const t=activeTab(); if(!t)return; const mode=t.clusterMode||"none"; if(mode==="none")return; const comp=mode==="component"?unionComponents(t.graph):{}; const n=t.graph.nodes.find(x=>x.id===id); if(!n)return; const cid=clusterKey(n,mode,comp); (t.collapsed=t.collapsed||new Set()).add(cid); renderGraph(); }

function renderGraph() {
  const t = activeTab();
  const container = $("#cy");
  $("#graphEmpty").hidden = !!(t && t.graph.nodes.length);
  if (!t || !t.graph.nodes.length) { if (cy) cy.elements().remove(); $("#graphStats").textContent="0 nodes · 0 edges"; return; }
  initCy();
  const model = computeRenderModel(t);
  const g = { nodes: model.nodes, edges: model.edges };
  const nodeById = {}; g.nodes.forEach(n=>nodeById[n.id]=n);
  // Perf mode: beyond this many rendered nodes, drop per-node SVG icons (heavy to
  // decode ×N) for solid coloured dots, and use a fast non-animated layout.
  const perf = g.nodes.length > 500;
  t._perf = perf;
  const els = [];
  g.nodes.forEach(n=>{
    if(n.meta){ els.push({ data:{ id:n.id, label:n.label, icon: perf?undefined:nodeIcon("group"), kc:kColor(n.kind), hc:bandColor(bandOf(n.risk)), size: metaSize(n.count) }, classes:"metanode"+(perf?" plain":"") }); return; }
    const band = n.band||bandOf(n.risk);
    const hot = band==="critical"||band==="high";
    els.push({ data:{ id:n.id, label:n.label, icon: perf?undefined:nodeIcon(n.kind), kc:kColor(n.kind), hc:bandColor(band),
      size: nodeSize(n.risk), bw:hot?2.5:1.5, halo:(hot&&!perf)?1:undefined }, classes:(n.hypothesis?"hyp ":"")+(perf?"plain":"") });
  });
  g.edges.forEach((e,i)=>{ if(nodeById[e.source]&&nodeById[e.target]) els.push({ data:{ id:"e"+i, source:e.source, target:e.target, type:e.type, w:edgeW(e.conf), kc:kColor((nodeById[e.source]||{}).kind) }, classes:e.hypothesis?"hyp":"" }); });
  cy.elements().remove(); cy.add(els);
  // If the graph container isn't visible yet (0×0), layout would be degenerate;
  // defer it to when the Graph view is shown (see showView).
  const cyEl=$("#cy"); const hidden = !cyEl || cyEl.offsetWidth===0 || cyEl.offsetHeight===0;
  // Even while hidden we lay out into an explicit bounding box (never a 0×0
  // container, which collapses every node to one point). A full-quality relayout
  // still runs once the view is shown.
  if(hidden){ t._needsRelayout=true; try{ cy.layout({name:"grid",animate:false,boundingBox:layoutBox(cy.nodes().length)}).run(); }catch(e){} }
  else { t._needsRelayout=false; runLayout(perf); }
  const full=t.graph; const clustered=(t.clusterMode||"none")!=="none";
  $("#graphStats").textContent = clustered ? `${g.nodes.length} shown · ${full.nodes.length} entities · ${full.edges.length} edges` : `${full.nodes.length} nodes · ${full.edges.length} edges`;
  const cs=$("#graphCluster"); if(cs) cs.value=t.clusterMode||"none";
  // Large graphs default to Overview (progressive reveal) once, so 10k+ nodes
  // don't dump as noise. The user can switch to Full/Risk/etc. anytime.
  if(!t._modeInit && full.nodes.length>=LARGE_GRAPH){ t._modeInit=true; setTimeout(()=>setGraphMode("overview"),150); }
  else { t._modeInit=true; syncModeButtons(); }
  renderLegend(); renderGraphFilters(); setTimeout(scheduleMinimap, 700);
}
// Deterministic layout area sized to the node count. Passed as `boundingBox` so
// the layout spreads nodes even when #cy is momentarily hidden or 0×0 (WKWebView
// on the desktop app reports zero size longer than Chromium) — without it, every
// node collapses onto a single point and the canvas looks empty. See runLayout.
function layoutBox(n){
  const cols = Math.max(1, Math.ceil(Math.sqrt(n * 1.8)));
  const rows = Math.max(1, Math.ceil(n / cols));
  return { x1:0, y1:0, w: cols*46, h: rows*46 };
}
function runLayout(perf) {
  if (!cy) return;
  const n = cy.nodes().length;
  if(perf===undefined) perf = n>500;
  const name = $("#graphLayout").value || "fcose";
  const box = layoutBox(n);
  let opts;
  if(perf){
    // Large graph: draft-quality, NON-animated fcose (or grid fallback) so the
    // WebView doesn't lock up laying out thousands of nodes.
    opts = n>2500
      ? { name:"grid", animate:false, padding:40, boundingBox:box }
      : { name:"fcose", quality:"draft", animate:false, randomize:true, nodeRepulsion:6000, idealEdgeLength:60, padding:40, samplingType:false, boundingBox:box };
  } else {
    opts = name==="fcose"
      ? { name:"fcose", animate:true, animationDuration:500, randomize:true, nodeRepulsion:8000, idealEdgeLength:70, padding:40, boundingBox:box }
      : { name, animate:true, padding:40, boundingBox:box };
  }
  try { cy.layout(opts).run(); } catch(e){ try{ cy.layout({name:"grid",animate:false,boundingBox:box}).run(); }catch(_){} }
}
function clearGraph(){ if(cy) cy.elements().remove(); }

function renderLegend() {
  const t = activeTab(); const nodes=(t?.graph.nodes||[]); const counts={};
  nodes.forEach(n=>counts[n.kind]=(counts[n.kind]||0)+1);
  const lg=$("#legend"); lg.innerHTML="";
  Object.entries(counts).sort((a,b)=>b[1]-a[1]).forEach(([k,c])=>{ const x=el("div","lg"); const d=el("span","kdot"); d.style.background=kColor(k); x.appendChild(d); const s=el("span"); s.innerHTML=`${k} <b>${c}</b>`; x.appendChild(s); lg.appendChild(x); });
}

// ---------- entity selection ----------
function nodeData(id){ const t=activeTab(); if(!t)return null;
  if(t._metas&&t._metas[id]){ const m=t._metas[id]; return { id:m.id, kind:m.kind, label:m.label, risk:m.risk, band:bandOf(m.risk), meta:true, members:m.members, attributes:{cluster:"collapsed", members:String(m.count)}, tags:["cluster"], sources:[] }; }
  return t.graph.nodes.find(n=>n.id===id); }
function selectNode(id) {
  const n = nodeData(id); if(!n) return;
  if (cy) { clearFocus(); cy.$(":selected").unselect(); const nel=cy.$id(id); if(nel&&nel.length){ nel.select(); focusNeighborhood(nel, true); } }
  const c=$("#context"); c.hidden=false;
  $("#ctxKind").textContent = n.kind + (n.sensitive?" · sensitive":"");
  $("#ctxName").textContent = n.label;
  const band=n.band||bandOf(n.risk);
  const deg=activeTab()?graphDegrees(activeTab().graph)[n.id]||0:0;
  const rc=n._rc!=null?n._rc:entResolution(n,deg); const q=n._q!=null?n._q:entQuality(n,deg);
  $("#ctxRisk").innerHTML = `<span class="band ${band}">${band} · ${(n.risk||0).toFixed(2)}</span><div class="risk-bar"><span style="width:${Math.round((n.risk||0)*100)}%;background:${bandColor(band)}"></span></div>`+
    (n.meta?"":`<div class="ctx-scores"><span class="score-badge ${scoreCls(rc)}">resolution ${pct(rc)}</span> <span class="qual-badge ${qualityLabel(q)}">quality ${qualityLabel(q)}</span> <span class="chip">${deg} conns</span></div>`);
  const tags=$("#ctxTags"); tags.innerHTML = n.tags.length?"":'<span class="chip">none</span>';
  n.tags.forEach(x=>{ const c=el("span","chip tag-chip"); c.appendChild(document.createTextNode(x));
    if(!n.meta){ const rm=el("span","tag-x"," ×"); rm.title="remove tag"; rm.addEventListener("click",e=>{ e.stopPropagation(); n.tags=n.tags.filter(t=>t!==x); selectNode(id); renderGraphFilters&&renderGraphFilters(); }); c.appendChild(rm); }
    tags.appendChild(c); });
  const addBtn=$("#ctxAddTag");
  if(addBtn){ addBtn.onclick=()=>{ if(n.meta){ toast("Can't tag a cluster","err"); return; }
    openModal("Add tag", `<div class="field">Tag<input id="newTag" placeholder="e.g. reviewed, priority, watchlist" /></div>`,
      [{label:"Cancel",cls:"ghost",act:closeModal},{label:"Add",cls:"primary",act:()=>{ const v=$("#newTag").value.trim().toLowerCase(); if(v){ n.tags=n.tags||[]; if(!n.tags.includes(v)){ n.tags.push(v); if(cy){const ne=cy.$id(id); if(ne&&ne.length&&!ne.hasClass("hyp")){}} } closeModal(); selectNode(id); renderGraphFilters&&renderGraphFilters(); pushNotif("entity","Tagged "+n.label+" · "+v); } else closeModal(); }}]);
    setTimeout(()=>$("#newTag")&&$("#newTag").focus(),40); }; }
  const meta=$("#ctxMeta"); meta.innerHTML=""; const es=Object.entries(n.attributes||{});
  if(!es.length) meta.innerHTML='<div class="empty">no metadata</div>';
  es.slice(0,24).forEach(([k,v])=>{ const r=el("div","row"); r.appendChild(el("span","k",k)); r.appendChild(el("span","v",String(v))); meta.appendChild(r); });
  const rels=$("#ctxRels"); rels.innerHTML="";
  const t=activeTab(); const related=(t?.graph.edges||[]).filter(e=>e.source===id||e.target===id);
  if(!related.length) rels.innerHTML='<div class="empty">no direct relations</div>';
  related.slice(0,50).forEach(e=>{ const other=e.source===id?e.target:e.source; const o=nodeData(other); if(!o)return;
    const r=el("div","rel"); r.innerHTML=`<span class="rt">${esc(e.type)}</span> ${e.source===id?"→":"←"} ${esc(o.label)}`;
    r.addEventListener("click",()=>{ selectNode(other); if(cy){const el2=cy.$id(other); if(el2) cy.animate({center:{eles:el2},duration:300}); } }); rels.appendChild(r); });
  const src=$("#ctxSources"); src.innerHTML = n.sources.length?"":'<span class="chip">—</span>'; n.sources.forEach(s=>src.appendChild(el("span","chip",s)));
  renderCtxTransforms(n.kind);
  showView("graph");
  if (cy){ const e2=cy.$id(id); if(e2) cy.animate({center:{eles:e2},duration:300}); }
}
$("#ctxClose").addEventListener("click",()=>$("#context").hidden=true);

// Focus an entity from the dashboard: switch to graph, isolate its neighborhood,
// select it and show its details.
function focusEntity(id, doIsolate){
  showView("graph");
  requestAnimationFrame(()=>{ initCy(); if(cy) cy.resize();
    if(doIsolate) isolate(id);
    selectNode(id);
    if(cy){ const e=cy.$id(id); if(e&&e.length) cy.animate({center:{eles:e},zoom:1.4,duration:400}); }
  });
}
// Isolate all critical entities (from the "Critical" KPI).
function isolateCritical(){
  const t=activeTab(); if(!t||!t.graph.nodes.length) return;
  const crit=new Set(t.graph.nodes.filter(n=>(n.band||bandOf(n.risk))==="critical").map(n=>n.id));
  if(!crit.size){ toast("No critical entities"); return; }
  // keep critical + their immediate neighbors for context
  const keep=new Set(crit); t.graph.edges.forEach(e=>{ if(crit.has(e.source))keep.add(e.target); if(crit.has(e.target))keep.add(e.source); });
  showView("graph");
  requestAnimationFrame(()=>{ initCy(); if(cy){ cy.resize(); cy.nodes().forEach(n=>n.style("display",keep.has(n.id())?"element":"none")); cy.fit(cy.nodes(":visible"),50); } });
  toast(`Isolated ${crit.size} critical entities`);
}
// KPI cards act as shortcuts.
["kcEntities","kcRels","kcCrit","kcActs"].forEach(id=>{ const c=$("#"+id); if(c) c.style.cursor="pointer"; });
$("#kcEntities")&&$("#kcEntities").addEventListener("click",()=>showView("entities"));
$("#kcRels")&&$("#kcRels").addEventListener("click",()=>{ showView("graph"); requestAnimationFrame(()=>{initCy(); if(cy){cy.resize();cy.fit(cy.elements(),50);}}); });
$("#kcCrit")&&$("#kcCrit").addEventListener("click",isolateCritical);
$$(".ctx-actions button").forEach(b=>b.addEventListener("click",()=>{
  const id = cy && cy.$(":selected").length ? cy.$(":selected")[0].id() : null;
  const n = id?nodeData(id):null; if(!n) return;
  const act=b.dataset.act;
  if(act==="isolate") isolate(id);
  else if(act==="neighbors") { if(cy) cy.fit(cy.$id(id).closedNeighborhood(),80); }
  else if(act==="alert"){ pushNotif("alert",`Alert on ${n.label}`); toast("Alert created"); }
  else if(act==="expand"){ askAbout(`Expand the investigation around "${n.label}" (${n.kind}). Propose linked entities and next leads.`); }
}));
function isolate(id){ const t=activeTab(); if(!t)return; const keep=new Set([id]); t.graph.edges.forEach(e=>{if(e.source===id)keep.add(e.target); if(e.target===id)keep.add(e.source);});
  if(cy){ cy.nodes().forEach(n=>{ n.style("display", keep.has(n.id())?"element":"none"); }); cy.fit(cy.nodes(":visible"),60); } toast("Isolated "+nodeData(id).label); }

// ---------- context menu ----------
let linkMode=null; // source id while connecting
async function openCtxMenu(x,y,id){ const m=$("#ctxmenu"); m.innerHTML="";
  const n=nodeData(id); if(!n)return;
  const add=(icon,label,fn,sep)=>{ if(sep){ const s=el("div","mi sep"); m.appendChild(s);} const mi=el("div","mi"); mi.innerHTML=svg(icon)+`<span>${esc(label)}</span>`; mi.addEventListener("click",()=>{fn();m.hidden=true;}); m.appendChild(mi); };
  if(n.meta){ add("graph",`Expand cluster (${n.members.length})`,()=>expandCluster(id)); add("fit","Focus cluster",()=>{ if(cy) cy.animate({fit:{eles:cy.$id(id),padding:120},duration:300}); });
    m.style.left=Math.min(x,window.innerWidth-230)+"px"; m.style.top=Math.min(y,window.innerHeight-120)+"px"; m.hidden=false; return; }
  add("entities","Open dossier",()=>selectNode(id));
  add("settings","Edit entity…",()=>editEntityModal(id));
  if((activeTab()?.clusterMode||"none")!=="none") add("boxes"in ICONS?"boxes":"entities","Collapse this cluster",()=>collapseNodeCluster(id));
  add("spark","Expand via AI",()=>askAbout(`Expand the investigation around "${n.label}" (${n.kind}). Propose linked entities and leads.`));
  add("fit","Isolate neighborhood",()=>isolate(id));
  add("graph","Focus neighbors",()=>{if(cy)cy.animate({fit:{eles:cy.$id(id).closedNeighborhood(),padding:80},duration:350});});
  add("graph","Neighborhood mode from here",()=>setGraphMode("neighborhood",{seed:id,hops:2}),true);
  add("graph","Find path from here…",()=>startPath(id));
  add("link","Connect to another node…",()=>startLink(id));
  add("alerts","Create alert",()=>{pushNotif("alert",`Alert on ${n.label}`);toast("Alert created");});
  add("trash","Remove node",()=>removeNode(id));
  // transforms submenu (installed, matching kind)
  let inst=[]; try{ inst=await api("/api/transforms"); }catch(e){}
  const match=inst.filter(t=>t.enabled && (!t.input_kinds.length || t.input_kinds.includes(n.kind)));
  if(match.length){ const s=el("div","mi sep"); m.appendChild(s); match.slice(0,8).forEach(t=>add("run",`Run: ${t.name}`,()=>{ cy.$(":selected").unselect(); cy.$id(id).select(); runTransformOnSelected(t); })); }
  m.style.left=Math.min(x,window.innerWidth-230)+"px"; m.style.top=Math.min(y,window.innerHeight-40-m.childElementCount*34)+"px"; m.hidden=false;
}
function startLink(id){ linkMode=id; let b=$("#linkmodeBanner"); if(!b){ b=el("div","linkmode"); b.id="linkmodeBanner"; b.textContent="Link mode: click a target node (Esc to cancel)"; $(".graph-wrap").appendChild(b);} b.hidden=false; }
function finishLink(targetId){ if(!linkMode) return; const s=linkMode, t=activeTab(); linkMode=null; const bn=$("#linkmodeBanner"); if(bn)bn.hidden=true;
  if(s===targetId||!t) return; t.graph.edges.push({source:s,target:targetId,type:"linked_by_analyst",conf:1.0}); renderGraph(); toast("Nodes connected"); }
function removeNode(id){ const t=activeTab(); if(!t)return; t.graph.nodes=t.graph.nodes.filter(n=>n.id!==id); t.graph.edges=t.graph.edges.filter(e=>e.source!==id&&e.target!==id); $("#context").hidden=true; renderGraph(); renderGraphFilters(); toast("Node removed"); }
window.addEventListener("click",()=>{ $("#ctxmenu").hidden=true; closeAllSelects(); });

// ---------- render all views ----------
let currentView="dashboard";
function showView(name){ currentView=name; $$(".view").forEach(v=>v.hidden=true); const v=$("#view-"+name); if(v)v.hidden=false;
  $$(".nav li").forEach(li=>li.classList.toggle("active",li.dataset.view===name));
  if(name==="graph"){
    // Double rAF: wait until the view is actually laid out (container has real
    // dimensions) before resize/layout/fit. A single frame or an early setTimeout
    // races the reflow on the desktop WebView, leaving the viewport zoomed on an
    // empty corner of the graph (looks blank). A trailing settle() re-fits after
    // the layout has committed.
    const settle=()=>{ initCy(); if(!cy) return; cy.resize();
      const t=activeTab();
      if(t && t._needsRelayout && cy.nodes().length){ t._needsRelayout=false; runLayout(t._perf); }
      if(cy.nodes().length) cy.fit(cy.elements(":visible"),50);
    };
    requestAnimationFrame(()=>requestAnimationFrame(()=>{ settle(); setTimeout(settle,140); }));
  } }
$$(".nav li").forEach(li=>li.addEventListener("click",()=>showView(li.dataset.view)));

function renderAll(){ renderGraph(); renderDashboard(); renderEntities(); renderReport(); renderTimeline(); renderAlerts(); renderSavedConnectors(); renderIntelligence(); }

// ===== decision-oriented metrics engine (shared by Dashboard + Entities + Intelligence) =====
function entResolution(n, deg){ let s=0.4; s+=Math.min(0.25,(n.sources?n.sources.length:0)*0.12);
  const attrs=Object.keys(n.attributes||{}).length; s+=Math.min(0.2,attrs*0.04);
  if((deg||0)>0)s+=0.1; if(n.tags&&n.tags.includes("manual"))s=Math.max(s,0.85);
  if(n.hypothesis||(n.tags&&n.tags.includes("hypothesis")))s=Math.min(s,0.35); return Math.min(1,Math.max(0.05,s)); }
function entQuality(n, deg){ let q=0.2; if(n.sources&&n.sources.length)q+=0.25; const attrs=Object.keys(n.attributes||{}).length; q+=Math.min(0.3,attrs*0.06);
  if((deg||0)>0)q+=0.15; else q-=0.05; if(n.label&&n.label.length>2)q+=0.1; return Math.max(0.05,Math.min(1,q)); }
function qualityLabel(q){ return q>=0.75?"clean":q>=0.5?"fair":q>=0.3?"incomplete":"weak"; }
function computeMetrics(g){ const deg=graphDegrees(g); const nodes=g.nodes;
  let highRisk=0,unresolved=0,missingSource=0,missingMeta=0,isolated=0,sensitive=0,hyp=0,confSum=0,qualSum=0;
  const groups={}; const bandCount={low:0,medium:0,high:0,critical:0}; const riskByKind={};
  nodes.forEach(n=>{ const dg=deg[n.id]||0; n._deg=dg; n._rc=entResolution(n,dg); n._q=entQuality(n,dg);
    confSum+=n._rc; qualSum+=n._q; const band=n.band||bandOf(n.risk); bandCount[band]=(bandCount[band]||0)+1;
    if(band==="critical"||band==="high")highRisk++; if(n._rc<0.5)unresolved++; if(!(n.sources&&n.sources.length))missingSource++;
    if(!Object.keys(n.attributes||{}).length)missingMeta++; if(dg===0)isolated++; if(n.sensitive)sensitive++;
    if(n.hypothesis||(n.tags&&n.tags.includes("hypothesis")))hyp++; riskByKind[n.kind]=(riskByKind[n.kind]||0)+(n.risk||0);
    const key=n.kind+"|"+n.label.trim().toLowerCase(); (groups[key]=groups[key]||[]).push(n.id); });
  const dupGroups=Object.values(groups).filter(a=>a.length>1); const duplicates=dupGroups.reduce((s,a)=>s+a.length,0);
  const N=nodes.length||1; const avgConf=confSum/N, avgQual=qualSum/N; const coverage=nodes.length?(nodes.length-isolated)/nodes.length:0;
  const sourceDiversity=new Set(nodes.flatMap(x=>x.sources||[])).size; const resolvedRatio=1-(unresolved/N);
  const readinessScore=nodes.length? (0.30*avgQual+0.25*resolvedRatio+0.20*avgConf+0.15*coverage+0.10*Math.min(1,sourceDiversity/5)) : 0;
  let readiness = !nodes.length?"insufficient": readinessScore>=0.72?"ready": readinessScore>=0.5?"needs-review": (highRisk>0&&avgConf<0.4?"conflicting":"insufficient");
  return { deg,highRisk,unresolved,missingSource,missingMeta,isolated,sensitive,hyp,duplicates,dupGroups,avgConf,avgQual,coverage,sourceDiversity,readinessScore,readiness,bandCount,riskByKind,total:nodes.length,edges:g.edges.length }; }
const pct=x=>Math.round((x||0)*100)+"%";

function renderDashboard(){ const t=activeTab();
  $("#dashTitle").textContent = t? t.project.name : "Command Center";
  $("#dashSub").textContent = t? `${t.project.domain} · investigation state, confidence, risk & next best action` : "Open or create a project to begin turning data into decisions.";
  const g=t?t.graph:{nodes:[],edges:[]}; const m=computeMetrics(g);
  const rdState=$("#rdState"); rdState.textContent={ready:"Ready for decision","needs-review":"Needs review",insufficient:"Insufficient data",conflicting:"Conflicting evidence"}[m.readiness]; rdState.className="rd-state "+m.readiness;
  $("#rdMeter").style.width=Math.round(m.readinessScore*100)+"%";
  $("#rdNote").textContent = !m.total?"No data yet — run an analysis or import a source." : `${pct(m.readinessScore)} · avg confidence ${pct(m.avgConf)} · data quality ${pct(m.avgQual)} · coverage ${pct(m.coverage)} · ${m.sourceDiversity} source(s).`;
  const rda=$("#rdActions"); rda.innerHTML="";
  const rdBtn=(lbl,fn)=>{ const b=el("button","btn "+(rda.children.length?"ghost":"primary"),lbl); b.addEventListener("click",fn); rda.appendChild(b); };
  if(m.total){ rdBtn("✦ Generate intelligence",()=>generateIntelligence()); rdBtn("Open graph",()=>showView("graph")); } else { rdBtn("▶ Run analysis",runModal); rdBtn("+ Add entity",addEntityModal); }
  const cards=[
    {v:pct(m.readinessScore),l:"Decision readiness",sub:m.readiness.replace("-"," "),bar:m.readinessScore,cls:m.readiness==="ready"?"":"warn",go:()=>{}},
    {v:pct(m.avgQual),l:"Data quality",sub:qualityLabel(m.avgQual),bar:m.avgQual,go:()=>{showView("entities");entityFilter="missing";renderEntities();}},
    {v:pct(m.avgConf),l:"Avg confidence",sub:m.avgConf>=0.6?"solid":"soft",bar:m.avgConf,go:()=>{showView("entities");entityFilter="lowconf";renderEntities();}},
    {v:m.highRisk,l:"High-risk entities",sub:"critical + high",cls:m.highRisk?"crit":"",go:()=>isolateCritical()},
    {v:m.unresolved,l:"Unresolved",sub:"low resolution conf.",cls:m.unresolved?"warn":"",go:()=>{showView("entities");entityFilter="lowconf";renderEntities();}},
    {v:m.missingSource+m.missingMeta,l:"Missing evidence",sub:`${m.missingSource} no-source · ${m.missingMeta} no-meta`,cls:(m.missingSource+m.missingMeta)?"warn":"",go:()=>{showView("entities");entityFilter="missing";renderEntities();}},
    {v:m.hyp,l:"Active hypotheses",sub:"AI-proposed, unconfirmed",go:()=>{showView("intelligence");renderIntelligence();}},
    {v:pct(m.coverage),l:"Graph coverage",sub:`${m.isolated} isolated`,bar:m.coverage,go:()=>showView("graph")},
  ];
  const dc=$("#dashCards"); dc.innerHTML="";
  cards.forEach(c=>{ const d=el("div","dcard "+(c.cls||"")); let h=`<div class="dc-v">${c.v}</div><div class="dc-l">${esc(c.l)}</div><div class="dc-sub">${esc(c.sub||"")}</div>`;
    if(c.bar!=null)h+=`<div class="dc-bar"><span style="width:${Math.round(c.bar*100)}%;background:${c.bar>=0.6?"var(--green)":c.bar>=0.35?"var(--amber)":"var(--red)"}"></span></div>`;
    d.innerHTML=h; d.addEventListener("click",c.go); dc.appendChild(d); });
  renderPriorities(m,g);
  const ro=$("#riskOverview"); ro.innerHTML=""; if(!m.total){ ro.innerHTML='<div class="empty">—</div>'; }
  else { [["critical",m.bandCount.critical||0],["high",m.bandCount.high||0],["medium",m.bandCount.medium||0],["low",m.bandCount.low||0]].forEach(([b,n])=>{ const row=el("div","riskrow"); row.innerHTML=`<span class="rr-l">${b}</span><div class="rr-track"><span style="width:${Math.round(n/m.total*100)}%;background:${bandColor(b)}"></span></div><span class="rr-n">${n}</span>`; if(n){row.style.cursor="pointer"; row.addEventListener("click",()=>{showView("graph");gfPreset=(b==="critical"||b==="high")?"crit":"all"; try{applyFilters();}catch(e){}});} ro.appendChild(row); });
    const topKinds=Object.entries(m.riskByKind).sort((a,b)=>b[1]-a[1]).slice(0,3).map(([k,v])=>`${k} ${v.toFixed(1)}`).join(" · ");
    if(topKinds){ const foot=el("div","muted"); foot.style.marginTop="8px"; foot.textContent="Top risk contributors: "+topKinds; ro.appendChild(foot); } }
  renderNextActions(m,g);
  const dq=$("#dqOverview"); dq.innerHTML=""; if(!m.total){ dq.innerHTML='<div class="empty">—</div>'; }
  else { [["no source",m.missingSource],["no metadata",m.missingMeta],["isolated",m.isolated],["likely duplicates",m.duplicates],["sensitive",m.sensitive]].forEach(([l,n])=>{ const row=el("div","dqrow"); const frac=m.total?n/m.total:0; row.innerHTML=`<span class="dq-l">${l}</span><div class="dq-track"><span style="width:${Math.round(frac*100)}%;background:${frac>0.3?"var(--red)":frac>0.1?"var(--amber)":"var(--green)"}"></span></div><span class="dq-n">${n}</span>`; dq.appendChild(row); });
    const note=el("div","muted"); note.style.marginTop="8px"; note.textContent="Low quality lowers Intelligence confidence — resolve in Entities."; dq.appendChild(note); }
  const sl=$("#signalsList"); sl.innerHTML="";
  const acts=t? [...t.project.activities].reverse().slice(0,8):[];
  if(!acts.length) sl.innerHTML='<div class="empty">—</div>';
  const sig={run:"spark",import:"sources",connect:"sources",ai:"spark",entity:"entities",transform:"run",report:"reports",note:"timeline",plugin:"operations",alert:"alerts"};
  acts.forEach(a=>{ const li=el("div","signal"); li.innerHTML=`<span class="s-ic">${svg(sig[a.kind]||"timeline")}</span><span class="label" style="flex:1">${esc(a.summary)}</span>`; sl.appendChild(li); });
  loadProjects();
}
function renderPriorities(m,g){ const w=$("#topPriorities"); w.innerHTML=""; const P=[];
  if(!m.total){ w.innerHTML='<div class="empty">Run an analysis or add entities to surface priorities.</div>'; return; }
  const crit=[...g.nodes].filter(n=>(n.band||bandOf(n.risk))==="critical").sort((a,b)=>b.risk-a.risk);
  if(crit.length) P.push({t:`Review ${crit.length} critical ${crit.length>1?"entities":"entity"}`,why:"Highest risk — may require escalation or protective action.",impact:"high",conf:"high",go:()=>isolateCritical()});
  if(m.duplicates) P.push({t:`Resolve ${m.duplicates} likely duplicates`,why:"Duplicates distort clusters and inflate risk.",impact:"medium",conf:"medium",go:()=>{showView("entities");entityFilter="dupes";renderEntities();}});
  if(m.missingMeta) P.push({t:`Enrich ${m.missingMeta} entities missing metadata`,why:"Sparse data lowers resolution confidence.",impact:"medium",conf:"high",go:()=>{showView("entities");entityFilter="missing";renderEntities();}});
  const bigHub=[...g.nodes].map(n=>({n,d:m.deg[n.id]||0})).sort((a,b)=>b.d-a.d)[0];
  if(bigHub&&bigHub.d>=8) P.push({t:`Inspect dense cluster around "${bigHub.n.label}"`,why:`${bigHub.d} connections — a structural hub worth examining.`,impact:"medium",conf:"medium",go:()=>focusEntity(bigHub.n.id,true)});
  if(m.readiness==="ready") P.push({t:"Generate action plan from intelligence",why:"Data supports a decision — synthesize the product.",impact:"high",conf:"high",go:()=>generateIntelligence()});
  if(!P.length) P.push({t:"Generate intelligence",why:"Baseline assessment of the current graph.",impact:"medium",conf:"medium",go:()=>generateIntelligence()});
  P.slice(0,6).forEach(p=>{ const d=el("div","prio"); d.innerHTML=`<span class="p-ic">${svg("alerts")}</span><div class="p-body"><div class="p-title">${esc(p.t)}</div><div class="p-why">${esc(p.why)}</div><div class="p-meta"><span class="p-tag">impact ${p.impact}</span><span class="p-tag">conf ${p.conf}</span></div></div>`; d.addEventListener("click",p.go); w.appendChild(d); });
}
function renderNextActions(m,g){ const w=$("#nextActions"); w.innerHTML=""; if(!m.total){ w.innerHTML='<div class="empty">—</div>'; return; }
  const A=[];
  if(m.unresolved) A.push({t:`Review ${m.unresolved} low-confidence entities`,go:()=>{showView("entities");entityFilter="lowconf";renderEntities();}});
  if(m.duplicates) A.push({t:`Merge ${m.duplicates} likely duplicates`,go:()=>{showView("entities");entityFilter="dupes";renderEntities();}});
  if(m.missingSource) A.push({t:`Trace source for ${m.missingSource} entities`,go:()=>{showView("entities");entityFilter="missing";renderEntities();}});
  if(m.highRisk) A.push({t:`Isolate ${m.highRisk} high-risk entities in graph`,go:()=>isolateCritical()});
  A.push({t:"Generate intelligence product",go:()=>generateIntelligence()});
  A.push({t:"Export PDF report",go:()=>{showView("reports");exportReportPdf();}});
  A.slice(0,6).forEach(a=>{ const li=el("div","li"); li.appendChild(el("span","label","▸ "+a.t)); const b=el("span","tag ok","do"); li.appendChild(b); li.addEventListener("click",a.go); w.appendChild(li); });
}
// ===== Entity Registry =====
let entityFilter="all"; let entSel=new Set();
function entStatus(t,n){ if(t._reviewed&&t._reviewed.has(n.id))return "reviewed"; if(n.tags&&n.tags.includes("manual"))return "manual"; if(n.hypothesis||(n.tags&&n.tags.includes("hypothesis")))return "hypothesis"; if((n._rc||0)<0.5)return "review"; return "resolved"; }
function scoreCls(x){ return x>=0.66?"hi":x>=0.4?"mid":"lo"; }
function entIntent(q,n,dupSet){ q=q.toLowerCase();
  const hay=(n.label+" "+n.kind+" "+(n.tags||[]).join(" ")+" "+Object.entries(n.attributes||{}).map(([k,v])=>k+" "+v).join(" ")).toLowerCase();
  const band=n.band||bandOf(n.risk);
  if(/high risk|critical/.test(q) && !(band==="high"||band==="critical")) return false;
  if(/low conf/.test(q) && (n._rc||0)>=0.5) return false;
  if(/(no|missing) source/.test(q) && (n.sources&&n.sources.length)) return false;
  if(/(no|missing) (meta|metadata)/.test(q) && Object.keys(n.attributes||{}).length) return false;
  if(/duplicat/.test(q) && !dupSet.has(n.id)) return false;
  if(/(isolated|no relation|no connection)/.test(q) && (n._deg||0)>0) return false;
  if(/sensitive/.test(q) && !n.sensitive) return false;
  // remaining free words must all be present somewhere
  const words=q.replace(/high risk|low conf(idence)?|no source|missing source|no metadata|missing metadata|duplicat\w*|isolated|no relation|no connection|sensitive|people|entities|accounts?|connected to|with|and|show/g,"").split(/\s+/).filter(Boolean);
  return words.every(w=>hay.includes(w));
}
function renderEntities(){ const t=activeTab(); const tb=$("#entitiesTable tbody"); if(!tb)return; tb.innerHTML="";
  const g=t?t.graph:{nodes:[],edges:[]}; const m=computeMetrics(g); const dupSet=new Set(m.dupGroups.flat());
  // summary cards
  const sc=$("#entSummary"); if(sc){ sc.innerHTML="";
    [["Total",m.total,""],["High risk",m.highRisk,"crit"],["Low confidence",m.unresolved,"warn"],["Likely duplicates",m.duplicates,"warn"]].forEach(([l,v,cls])=>{ const d=el("div","dcard "+cls); d.innerHTML=`<div class="dc-v">${v}</div><div class="dc-l">${l}</div>`; sc.appendChild(d); }); }
  // filter chips
  const filters=[["all","All",m.total],["highrisk","High risk",m.highRisk],["lowconf","Low confidence",m.unresolved],["dupes","Duplicates",m.duplicates],["missing","Missing evidence",m.missingSource+m.missingMeta],["norel","No relations",m.isolated],["hub","High degree",g.nodes.filter(n=>(m.deg[n.id]||0)>=8).length],["sensitive","Sensitive",m.sensitive],["manual","Manual",g.nodes.filter(n=>n.tags&&n.tags.includes("manual")).length],["review","Needs review",g.nodes.filter(n=>entStatus(t,n)==="review").length]];
  const fw=$("#entFilters"); if(fw){ fw.innerHTML=""; filters.forEach(([id,label,cnt])=>{ const c=el("button","efilter"+(entityFilter===id?" active":"")); c.innerHTML=`${esc(label)}<span class="cnt">${cnt}</span>`; c.addEventListener("click",()=>{entityFilter=id;renderEntities();}); fw.appendChild(c); }); }
  // filter + search
  const q=($("#entSearch")&&$("#entSearch").value||"").trim();
  let rows=[...g.nodes];
  const pass=n=>{ const band=n.band||bandOf(n.risk); switch(entityFilter){
    case "highrisk": return band==="high"||band==="critical"; case "lowconf": return (n._rc||0)<0.5;
    case "dupes": return dupSet.has(n.id); case "missing": return !(n.sources&&n.sources.length)||!Object.keys(n.attributes||{}).length;
    case "norel": return (n._deg||0)===0; case "hub": return (m.deg[n.id]||0)>=8; case "sensitive": return !!n.sensitive;
    case "manual": return n.tags&&n.tags.includes("manual"); case "review": return entStatus(t,n)==="review"; default: return true; } };
  rows=rows.filter(pass); if(q) rows=rows.filter(n=>entIntent(q,n,dupSet));
  rows.sort((a,b)=>(b.risk||0)-(a.risk||0));
  const shown=rows.slice(0,400);
  shown.forEach(n=>{ const band=n.band||bandOf(n.risk); const tr=el("tr");
    const cb=el("td"); const box=el("input"); box.type="checkbox"; box.checked=entSel.has(n.id); box.addEventListener("click",e=>{e.stopPropagation(); if(box.checked)entSel.add(n.id); else entSel.delete(n.id); updateBulkBar();}); cb.appendChild(box); tr.appendChild(cb);
    const ent=el("td"); const wrap=el("div","ent-ent"); const ic=el("span","eic"); ic.style.color=kColor(n.kind); ic.innerHTML=svg2(n.kind); wrap.appendChild(ic); wrap.appendChild(el("span","label",n.label)); ent.appendChild(wrap); tr.appendChild(ent);
    tr.appendChild(el("td",null,n.kind));
    const rk=el("td"); rk.appendChild(el("span","band "+band,(n.risk||0).toFixed(2))); tr.appendChild(rk);
    const cf=el("td"); cf.appendChild(el("span","score-badge "+scoreCls(n._rc),pct(n._rc))); tr.appendChild(cf);
    const ql=el("td"); ql.appendChild(el("span","qual-badge "+qualityLabel(n._q),qualityLabel(n._q))); tr.appendChild(ql);
    tr.appendChild(el("td",null,String(m.deg[n.id]||0)));
    tr.appendChild(el("td",null,String((n.sources||[]).length)||"0"));
    const st=entStatus(t,n); const stc=el("td"); stc.appendChild(el("span","st-badge "+st, st==="review"?"needs review":st)); tr.appendChild(stc);
    tr.addEventListener("click",()=>selectNode(n.id)); tb.appendChild(tr); });
  window._entShownIds = shown.map(n=>n.id);
  const foot=$("#entFoot"); if(foot) foot.textContent = `${rows.length} matching · showing ${shown.length}`+(rows.length>shown.length?` (capped) · refine with filters/search`:"");
  const selAll=$("#entSelAll"); if(selAll) selAll.checked = shown.length>0 && shown.every(n=>entSel.has(n.id));
  updateBulkBar();
}
// small icon reuse from graph glyphs (dark on kind color)
function svg2(kind){ const p=ENTITY_GLYPH[kind]||ENTITY_GLYPH.unknown; return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${p}</svg>`; }
function updateBulkBar(){ const bar=$("#entBulkBar"); if(!bar)return; bar.hidden=entSel.size===0; $("#entSelCount").textContent=entSel.size+" selected"; }
$("#entSearch")&&$("#entSearch").addEventListener("input",()=>{ clearTimeout(window._es); window._es=setTimeout(renderEntities,180); });
$("#entSelAll")&&$("#entSelAll").addEventListener("change",e=>{ const ids=window._entShownIds||[]; if(e.target.checked) ids.forEach(id=>entSel.add(id)); else ids.forEach(id=>entSel.delete(id)); renderEntities(); });
$$("#entBulkBar button").forEach(b=>b.addEventListener("click",()=>{ const act=b.dataset.bulk; const t=activeTab(); if(!t)return; const ids=[...entSel];
  if(act==="clear"){ entSel.clear(); renderEntities(); return; }
  if(!ids.length){ toast("Select entities first","err"); return; }
  if(act==="graph"){ if(cy){ const keep=new Set(ids); cy.nodes().forEach(nd=>nd.style("display",keep.has(nd.id())?"element":"none")); cy.edges().forEach(ed=>ed.style("display",(keep.has(ed.source().id())&&keep.has(ed.target().id()))?"element":"none")); showView("graph"); requestAnimationFrame(()=>{initCy();cy.resize();cy.fit(cy.nodes(":visible"),70);}); toast(`Isolated ${ids.length} in graph`,"ok"); } }
  else if(act==="review"){ t._reviewed=t._reviewed||new Set(); ids.forEach(id=>t._reviewed.add(id)); toast(`${ids.length} marked reviewed`,"ok"); renderEntities(); }
  else if(act==="tag"){ openModal("Bulk tag",`<div class="field">Tag<input id="btag" placeholder="e.g. reviewed-batch-1"/></div>`,[{label:"Cancel",cls:"ghost",act:closeModal},{label:"Apply",cls:"primary",act:()=>{ const tag=$("#btag").value.trim(); if(tag){ ids.forEach(id=>{ const n=t.graph.nodes.find(x=>x.id===id); if(n){ n.tags=n.tags||[]; if(!n.tags.includes(tag))n.tags.push(tag);} }); } closeModal(); renderEntities(); toast("Tagged","ok"); }}]); }
  else if(act==="intel"){ const labels=ids.map(id=>{ const n=t.graph.nodes.find(x=>x.id===id); return n?n.label:""; }).filter(Boolean).slice(0,40); generateIntelligence("Focus the intelligence on these selected entities: "+labels.join(", ")); }
}));
function renderReport(){ const t=activeTab(); const b=$("#reportBody");
  const inv=t&&t.graph.meta&&t.graph.meta.investigation; const gov=t&&t.graph.meta&&t.graph.meta.governance;
  if(!inv){ b.innerHTML='<div class="empty">Run an analysis to generate a brief.</div>'; return; }
  let h=""; if(inv.summary) h+=`<p>${esc(inv.summary)}</p>`;
  const arr=(title,items)=>{ if(items&&items.length){ h+=`<h3>${title}</h3><ul>`; items.forEach(x=>h+=`<li>${esc(typeof x==="string"?x:JSON.stringify(x))}</li>`); h+="</ul>"; } };
  arr("Key findings",inv.key_findings); arr("Strongest leads",inv.strongest_leads); arr("Protective actions",inv.protective_actions);
  if(inv.next_steps&&inv.next_steps.length){ h+="<h3>Next steps</h3><ul>"; inv.next_steps.forEach(s=>h+=`<li>${esc(s.action||"")}${s.requires_authorization?' <span class="auth">(requires authorization)</span>':''}</li>`); h+="</ul>"; }
  if(gov){ const s=gov.audit_summary||{},r=gov.retention||{}; h+="<h3>Governance</h3><ul>"; if(s.summary)h+=`<li>${esc(s.summary)}</li>`; if(r.retention_days!=null)h+=`<li>Retention: ${r.retention_days} days → disposal ${String(r.disposal_date||"").slice(0,10)}</li>`; h+="</ul>"; }
  b.innerHTML=h; }
function renderTimeline(){ const t=activeTab(); const w=$("#timeline"); w.innerHTML="";
  const audit=(t&&t.graph.meta&&t.graph.meta.audit)||[];
  if(!audit.length){ w.innerHTML='<div class="empty">Run an analysis to populate.</div>'; return; }
  audit.forEach(e=>{ const it=el("div","tl-item"); it.appendChild(el("div","tl-time",String(e.timestamp||"").replace("T"," ").slice(0,19))); it.appendChild(el("div","tl-title",`${e.action_performed} · ${e.stage}`)); it.appendChild(el("div","tl-desc",e.entity_scope||"")); w.appendChild(it); }); }
function renderAlerts(){ const t=activeTab(); const a=$("#alertsList"); a.innerHTML="";
  const risk=t&&t.graph.meta&&t.graph.meta.risk; const items=(risk&&risk.assessments||[]).filter(x=>x.requires_human_review||x.risk_band==="critical");
  if(!items.length){ a.innerHTML='<div class="empty">No flagged items.</div>'; }
  items.slice(0,60).forEach(x=>{ const li=el("div","li"); const l=el("div","l"); const d=el("span","kdot"); d.style.background=kColor(x.entity_kind); l.appendChild(d); l.appendChild(el("span","label",`${x.entity_label} — ${x.recommended_action}`)); li.appendChild(l); li.appendChild(el("span","band "+(x.risk_band||"high"),x.risk_band||"high")); li.addEventListener("click",()=>selectNode(x.entity_id)); a.appendChild(li); });
}

// ---------- run analysis ----------
function runModal(){
  const t=activeTab();
  const domainOpts = state.domains.map(d=>`<option value="${d.slug}" ${t&&t.project.domain===d.slug?"selected":""}>${esc(d.title)}</option>`).join("");
  // group data types by category
  const cats={}; state.dataTypes.forEach(dt=>{ (cats[dt.category||"Other"]=cats[dt.category||"Other"]||[]).push(dt.slug); });
  let typeOpts='<option value="">Auto-classify</option>';
  Object.entries(cats).forEach(([c,list])=>{ typeOpts+=`<optgroup label="${esc(c)}">`+list.map(s=>`<option value="${s}">${s}</option>`).join("")+"</optgroup>"; });
  openModal("Run analysis", `
    <div class="field">Business vertical<select id="rDomain" class="select">${domainOpts}</select></div>
    <div class="field">Data type (category → type, or auto)<select id="rType" class="select">${typeOpts}</select></div>
    <div class="field">AI provider<select id="rProvider" class="select">
      <option value="auto">Auto — smart routing (Opus/Sonnet ⇄ Codex)</option><option value="claude">Claude (Opus/Sonnet)</option><option value="codex">Codex (gpt-5.5)</option><option value="mock">Offline mock</option></select></div>
    <div class="field">Input source(s)<div style="display:flex;gap:8px"><input id="rInputs" placeholder="/path/to/data.csv or .json  (or Browse)" style="flex:1" /><button class="btn ghost" id="rBrowse">Browse…</button></div></div>
    <div class="field">Max records (graph cap)<input id="rMax" type="number" value="4000" /></div>
    ${MODE==="mock"?'<div class="modal-note">Preview mode: loads the embedded sample.</div>':''}
  `,[
    {label:"Cancel",cls:"ghost",act:closeModal},
    {label:"▶ Run",cls:"primary",act:doRun}
  ]);
  setTimeout(()=>{ const b=$("#rBrowse"); if(b) b.addEventListener("click",()=>pickServerPath(path=>{ const cur=$("#rInputs").value.trim(); $("#rInputs").value=(cur?cur+" ":"")+path; }, {title:"Choose input (file or folder of media)", folders:true, accept:".csv,.tsv,.json,.jsonl,.ndjson,.png,.jpg,.jpeg,.gif,.webp,.mp4,.mov,.mp3,.wav,.pdf"})); },40);
  setTimeout(()=>{ if($("#rProvider")) $("#rProvider").value=state.provider; },30);
}
async function doRun(){
  const t=activeTab(); if(!t){ toast("Open a project first","err"); return; }
  const params={ inputs:$("#rInputs").value.split(/\s+/).filter(Boolean), domain:$("#rDomain").value,
    dataType:$("#rType").value||null, provider:$("#rProvider").value, maxRecords:parseInt($("#rMax").value)||4000,
    offline:$("#rProvider").value==="mock", projectId:t.project.id };
  state.provider=params.provider; $("#providerPill").textContent="provider: "+state.provider;
  closeModal();
  if(MODE!=="mock" && !params.inputs.length){ toast("Provide an input path","err"); return; }
  setSync("busy","running"); toast("Running pipeline…");
  try {
    const result = await runJob("run",params);
    t.result=result; t.graph=consolidatedToGraph(result);
    t.project = await api(`/api/projects/get?id=${encodeURIComponent(t.project.id)}`).catch(()=>t.project);
    setSync("ok","complete"); renderAll(); showView("graph"); setTimeout(()=>{initCy(); if(cy)cy.fit(cy.elements(),50);},700);
    pushNotif("run",`Analysis complete: ${t.graph.nodes.length} entities`);
    toast(`Done — ${t.graph.nodes.length} entities, ${t.graph.edges.length} relationships`,"ok");
  } catch(e){ setSync("err","failed"); toast("Run failed: "+e.message,"err"); }
}
$("#btnRun").addEventListener("click",runModal);
$("#btnRun2").addEventListener("click",runModal);

// ---------- AI copilot ----------
function openAsk(){ $("#askDock").hidden=false; showView("graph"); $("#askText").focus();
  const log=$("#askLog");
  if(!log._hinted){ log._hinted=true; const h=el("div","ask-msg a");
    h.innerHTML = state.provider==="mock"
      ? "✦ Provider is <b>offline mock</b> — switch to Claude/Codex in Settings for live intelligence."
      : `✦ Copilot ready (provider: <b>${esc(state.provider)}</b>). Ask about the current graph.`;
    log.appendChild(h); }
}
// The graph toolbar's Ask (btnAsk2) opens the graph-only copilot dock.
$("#btnAsk2").addEventListener("click",openAsk);

// ---------- Global Ask AI (top bar): summarize / navigate / act anywhere ----------
// Distinct from the graph AI Copilot (which stays inside the graph). This is an
// app-wide assistant: it can jump to any view, run actions, and summarize.
function openGlobalAsk(){ $("#askBackdrop").hidden=false; const i=$("#askGlobalInput"); i.value=""; i.focus(); }
function closeGlobalAsk(){ $("#askBackdrop").hidden=true; }
$("#btnAsk")&&$("#btnAsk").addEventListener("click",openGlobalAsk);
$("#askBackdrop")&&$("#askBackdrop").addEventListener("click",e=>{ if(e.target===$("#askBackdrop")) closeGlobalAsk(); });
$("#askGlobalInput")&&$("#askGlobalInput").addEventListener("keydown",e=>{ if(e.key==="Enter"){ e.preventDefault(); runGlobalAsk($("#askGlobalInput").value); } });

// Deterministic intent router — navigation + core actions, offline. Returns a
// handled result {msg, run:fn} or null if it's a free-form question for the LLM.
function globalIntent(q){
  const s=q.toLowerCase().trim();
  const views={dashboard:["dashboard","overview","command","início","inicio","home"],graph:["graph","grafo","network","rede"],intelligence:["intelligence","inteligência","inteligencia","assessment","analysis","análise"],entities:["entities","entidades","registry","registro"],timeline:["timeline","linha do tempo","tempo"],alerts:["alerts","alertas"],reports:["reports","relatório","relatorios","relatórios","pdf"],settings:["settings","config","configuração","ajustes"]};
  // navigation
  if(/\b(go to|open|take me|ir para|abrir|vá para|va para|mostrar? a aba|show me the)\b/.test(s) || /^(dashboard|graph|intelligence|entities|timeline|alerts|reports|settings)$/.test(s)){
    for(const [v,keys] of Object.entries(views)){ if(keys.some(k=>s.includes(k))){ return { msg:`Opening ${v}.`, run:()=>{ if(v==="settings")openSettingsTab("account"); else { showView(v); if(v==="intelligence")renderIntelligence(); if(v==="entities")renderEntities(); if(v==="reports"){renderReport();renderReports();} } } }; } }
  }
  // core actions
  if(/\b(run|rode|rodar|executar|analy[sz]e|análise)\b/.test(s) && /\b(analysis|análise|pipeline|dados|data)\b/.test(s)) return { msg:"Opening the Run dialog.", run:runModal };
  if(/\b(generate|gerar|synthesi[sz]e|produce)\b/.test(s) && /\b(intelligence|inteligência|inteligencia|assessment|brief)\b/.test(s)) return { msg:"Generating intelligence…", run:()=>{ showView("intelligence"); generateIntelligence(); } };
  if(/\b(export|exportar|gerar|generate|baixar|download)\b/.test(s) && /\b(pdf|report|relatório|relatorio)\b/.test(s)) return { msg:"Generating a PDF report…", run:()=>{ showView("reports"); exportReportPdf(); } };
  if(/\b(isolate|isolar|show|mostr)\b.*\b(critical|crítico|critico|high[- ]?risk|alto risco)\b/.test(s) || /\b(high[- ]?risk|alto risco)\b/.test(s)) return { msg:"Isolating high-risk entities in the graph.", run:()=>{ setGraphMode("risk"); } };
  if(/\b(new|novo|criar|create)\b.*\b(project|projeto)\b/.test(s)) return { msg:"Opening New Project.", run:newProjectModal };
  if(/\b(add|adicionar|criar)\b.*\b(entit|entidade|node|nó)\b/.test(s)) return { msg:"Opening Add Entity.", run:addEntityModal };
  return null;
}
async function runGlobalAsk(q){ q=(q||"").trim(); if(!q) return;
  const body=$("#askGlobalBody"); body.innerHTML="";
  const u=el("div","askbar-msg u","» "+q); body.appendChild(u);
  const intent=globalIntent(q);
  if(intent){ const a=el("div","askbar-msg a",intent.msg); body.appendChild(a); setTimeout(()=>{ closeGlobalAsk(); intent.run(); }, 350); return; }
  // Free-form → summarize/answer using the active project's graph, then optionally act.
  const t=activeTab();
  const think=el("div","askbar-msg a","✦ thinking…"); body.appendChild(think);
  try{
    const graph=t?{nodes:t.graph.nodes,edges:t.graph.edges}:{nodes:[],edges:[]};
    const res=await runJob("ask",{question:q+"\n\n(You are the app-wide assistant. Summarize clearly for a non-technical user; if a specific view/action would help, mention it.)", domain:t?t.project.domain:"generic", provider:state.provider, graph, aiInstructions:t?t.project.ai_instructions:""});
    think.remove(); const a=el("div","askbar-msg a"); let h=`<div>${esc(res.answer||"(no answer)")}</div>`;
    if(res.key_points&&res.key_points.length) h+="<ul>"+res.key_points.slice(0,5).map(p=>`<li>${esc(p)}</li>`).join("")+"</ul>";
    a.innerHTML=h;
    // offer quick follow-through
    const goIntel=el("span","askbar-do","✦ Open Intelligence"); goIntel.addEventListener("click",()=>{closeGlobalAsk();showView("intelligence");renderIntelligence();}); a.appendChild(goIntel);
    if(res.focus&&res.focus.action&&res.focus.action!=="none"){ const goG=el("span","askbar-do","Show in graph"); goG.addEventListener("click",()=>{closeGlobalAsk();applyFocus(res.focus);}); a.appendChild(goG); }
    body.appendChild(a);
  }catch(e){ think.remove();
    // offline fallback: local filter or a plain message
    const local=localFilterAnswer(q,t); const a=el("div","askbar-msg a");
    if(local){ a.innerHTML=local.html; body.appendChild(a); if(local.ids.length){ const g=el("span","askbar-do","Show in graph"); g.addEventListener("click",()=>{closeGlobalAsk();applyLocalFocus(local.ids);}); a.appendChild(g);} }
    else { a.innerHTML="✦ No AI backend reachable — I can still navigate and run actions. Try “go to intelligence” or “export a PDF report”."; body.appendChild(a); }
  }
}
$("#askClose").addEventListener("click",()=>$("#askDock").hidden=true);
$("#askSend").addEventListener("click",()=>askAbout($("#askText").value));
$("#askText").addEventListener("keydown",e=>{ if(e.key==="Enter"&&(e.metaKey||e.ctrlKey)){ e.preventDefault(); askAbout($("#askText").value);} });
async function askAbout(q){
  q=(q||"").trim(); if(!q) return;
  const t=activeTab(); openAsk(); $("#askText").value="";
  const log=$("#askLog"); const u=el("div","ask-msg u",q); log.appendChild(u); log.scrollTop=log.scrollHeight;
  const thinking=el("div","ask-msg a","✦ thinking…"); log.appendChild(thinking); log.scrollTop=log.scrollHeight;
  try {
    const graph = t? {nodes:t.graph.nodes, edges:t.graph.edges}:{nodes:[],edges:[]};
    const res = await runJob("ask",{question:q, domain:t?t.project.domain:"generic", provider:state.provider, graph, aiInstructions:t?t.project.ai_instructions:""});
    thinking.remove();
    const a=el("div","ask-msg a");
    let h=`<div>${esc(res.answer||"(no answer)")}</div>`;
    if(res.key_points&&res.key_points.length){ h+='<ul class="pts">'+res.key_points.map(p=>`<li>${esc(p)}</li>`).join("")+'</ul>'; }
    if(res.recommended_actions&&res.recommended_actions.length){ h+='<ul class="pts">'+res.recommended_actions.map(p=>`<li>▸ ${esc(p)}</li>`).join("")+'</ul>'; }
    const adds=(res.entities&&res.entities.length)||(res.relationships&&res.relationships.length);
    if(adds){ const n=(res.entities||[]).length,r=(res.relationships||[]).length; h+=`<div class="adds" id="addProp">＋ Add ${n} entities / ${r} relations to graph</div>`; }
    a.innerHTML=h; log.appendChild(a); log.scrollTop=log.scrollHeight;
    const plain=[res.answer||"", ...(res.key_points||[]), ...(res.recommended_actions||[]).map(p=>"• "+p)].filter(Boolean).join("\n");
    addCopyBtn(a, plain);
    if(adds){ $("#addProp").addEventListener("click",()=>mergeProposals(res)); }
    applyFocus(res.focus);
    pushNotif("ai","AI copilot answered a query");
  } catch(e){
    thinking.remove();
    // Deterministic fallback: many asks are simple "show/filter X" requests that
    // don't need an LLM. Try a local graph filter before surfacing the error.
    const local=localFilterAnswer(q, t);
    const a=el("div","ask-msg a");
    if(local){ a.innerHTML=local.html; log.appendChild(a); log.scrollTop=log.scrollHeight; applyLocalFocus(local.ids); }
    else { const noProvider=/spawn|installed|providers failed/.test(e.message||"");
      a.innerHTML = noProvider
        ? `✦ No AI backend reachable (Claude/Codex not found on PATH). I answered locally where I could. Set a provider in Settings, or run <code>cortex serve</code> from a terminal where <code>claude</code>/<code>codex</code> are on PATH.`
        : "✦ error: "+esc(e.message);
      log.appendChild(a); }
    log.scrollTop=log.scrollHeight;
  }
}
// Handle simple "show/filter/only X" asks without an LLM: match nodes whose
// label/attributes contain the query's salient tokens, and isolate them.
function localFilterAnswer(q, t){
  if(!t||!t.graph.nodes.length) return null;
  const ql=q.toLowerCase();
  if(!/\b(show|filter|only|mostr|mostre|exib|list|find|encontr|quais|onde|filtr)\b/.test(ql)) {
    // still allow bare token queries like "protonmail"
  }
  // salient tokens: things with . or @, or words >2 chars that aren't stopwords
  const stop=new Set(["show","only","that","are","the","with","and","or","me","os","que","sao","são","no","na","graph","grafo","mostre","mostrar","exibir","quais","onde","from","filter","filtre","find","list","de","da","do","as","um","uma"]);
  const toks=(q.match(/[A-Za-z0-9._@\-]+/g)||[]).map(s=>s.toLowerCase()).filter(s=>s.length>2 && !stop.has(s));
  if(!toks.length) return null;
  const hay=n=>(n.label+" "+n.kind+" "+(n.tags||[]).join(" ")+" "+Object.entries(n.attributes||{}).map(([k,v])=>k+" "+v).join(" ")).toLowerCase();
  const matches=t.graph.nodes.filter(n=>toks.some(tk=>hay(n).includes(tk)));
  if(!matches.length) return { html:`✦ (local) No entities match: ${esc(toks.join(", "))}.`, ids:[] };
  const ids=matches.map(n=>n.id);
  const sample=matches.slice(0,12).map(n=>esc(n.label)).join(", ");
  return { html:`✦ (local filter) <b>${matches.length}</b> entit${matches.length>1?"ies":"y"} match <b>${esc(toks.join(", "))}</b> — isolated in the graph.<div class="pts muted" style="margin-top:4px;font-size:11px">${sample}${matches.length>12?" …":""}</div>`, ids };
}
function applyLocalFocus(ids){ if(!ids||!ids.length) return; const t=activeTab(); if(!t) return; showView("graph");
  const keep=new Set(ids);
  const need=()=>{ initCy(); return cy && ids.some(id=>cy.$id(id).length); };
  // If the graph is clustered (Overview) the individual nodes aren't rendered —
  // rebuild uncollapsed so we can isolate the matches.
  if((t.clusterMode||"none")!=="none" || !need()){ t.clusterMode="none"; t.graphMode="full"; $("#graphCluster")&&($("#graphCluster").value="none"); renderGraph(); }
  setTimeout(()=>requestAnimationFrame(()=>{ initCy(); if(!cy)return; cy.resize();
    cy.nodes().forEach(n=>n.style("display",keep.has(n.id())?"element":"none"));
    cy.edges().forEach(ed=>ed.style("display",(keep.has(ed.source().id())&&keep.has(ed.target().id()))?"element":"none"));
    if(cy.nodes(":visible").length) cy.fit(cy.nodes(":visible"),70);
  }), 350);
}
function mergeProposals(res){
  const t=activeTab(); if(!t) return;
  const byLabel={}; t.graph.nodes.forEach(n=>byLabel[n.label.toLowerCase()]=n.id);
  const newIds=[];
  (res.entities||[]).forEach(e=>{ const key=(e.label||"").toLowerCase(); if(!key)return; if(byLabel[key]){ newIds.push(byLabel[key]); return; }
    const id="ai-"+Math.abs(hashStr(key)); byLabel[key]=id; newIds.push(id);
    t.graph.nodes.push({id,kind:(e.kind||"unknown"),label:e.label,risk:0.4,band:"medium",attributes:e.attributes||{},tags:["hypothesis"],sources:["ai-copilot"],hypothesis:!!e.hypothesis}); });
  (res.relationships||[]).forEach(r=>{ const s=byLabel[(r.source||"").toLowerCase()],tg=byLabel[(r.target||"").toLowerCase()]; if(s&&tg) t.graph.edges.push({source:s,target:tg,type:r.type||"related",conf:r.confidence||0.5,hypothesis:!!r.hypothesis}); });
  if(!newIds.length){ toast("Nothing new to add","err"); return; }
  // Clear any active isolate/filter so the additions are actually visible, then
  // center + flash the new nodes with their neighborhood.
  if(t.clusterMode&&t.clusterMode!=="none"){ t.clusterMode="none"; }
  renderGraph(); showView("graph");
  requestAnimationFrame(()=>{ initCy(); if(!cy)return; cy.resize(); cy.nodes().style("display","element"); cy.edges().style("display","element");
    const eles=cy.collection(); newIds.forEach(id=>{ const e=cy.$id(id); if(e&&e.length){ eles.merge(e); eles.merge(e.connectedEdges().connectedNodes()); } });
    if(eles.length){ cy.animate({fit:{eles:eles, padding:90}, duration:400}); flashFresh(newIds); }
  });
  pushNotif("ai",`Added ${newIds.length} AI-proposed entities`);
  toast(`Added ${newIds.length} entities to graph`,"ok");
}
function hashStr(s){ let h=0; for(let i=0;i<s.length;i++){ h=(h*31+s.charCodeAt(i))|0; } return h; }
// Apply an AI-returned focus directly to the graph (isolate/highlight), no manual filtering.
function applyFocus(focus){ if(!focus||!cy||focus.action==="none") return; const t=activeTab(); if(!t) return;
  const labels=new Set((focus.entity_labels||[]).map(s=>String(s).toLowerCase()));
  const kinds=new Set((focus.kinds||[]).map(s=>String(s).toLowerCase()));
  const minRisk=typeof focus.min_risk==="number"?focus.min_risk:null;
  const match=n=>{ let ok=false; if(labels.size&&labels.has(n.label.toLowerCase()))ok=true; if(kinds.size&&kinds.has(n.kind.toLowerCase()))ok=true; if(minRisk!=null&&(n.risk||0)>=minRisk)ok=true; if(!labels.size&&!kinds.size&&minRisk==null)ok=true; return ok; };
  const keep=new Set(); t.graph.nodes.forEach(n=>{ if(match(n)) keep.add(n.id); });
  if(!keep.size) return;
  // Grow to include the immediate neighbors of matched nodes so the isolated
  // view is a coherent subgraph, not disconnected dots.
  t.graph.edges.forEach(e=>{ if(keep.has(e.source))keep.add(e.target); if(keep.has(e.target))keep.add(e.source); });
  showView("graph"); requestAnimationFrame(()=>{ initCy(); if(!cy)return; cy.resize();
    // Always ISOLATE (hide the rest) so the analyst gets a clean subgraph.
    cy.elements().removeClass("faded");
    cy.nodes().forEach(nd=>nd.style("display", keep.has(nd.id())?"element":"none"));
    cy.edges().forEach(ed=>ed.style("display",(keep.has(ed.source().id())&&keep.has(ed.target().id()))?"element":"none"));
    cy.fit(cy.nodes(":visible"),70);
  });
  $("#graphStats").textContent = `${keep.size} isolated · ${t.graph.nodes.length} entities · use Reset to restore`;
  toast(`AI isolated ${keep.size} entities — Reset to restore full graph`,"ok");
}

// ---------- connectors ----------
const CONNECTORS=[
  {kind:"csv",name:"CSV / TSV file",desc:"Import a delimited file and auto-process it."},
  {kind:"json",name:"JSON / JSONL",desc:"Import JSON records and expand classification."},
  {kind:"postgres",name:"PostgreSQL",desc:"Connect by host/IP, user & password; run a query."},
  {kind:"mysql",name:"MySQL / MariaDB",desc:"Connect by host/IP, user & password; run a query."},
  {kind:"bigquery",name:"Google BigQuery",desc:"Query BigQuery via the bq CLI."},
  {kind:"datalake",name:"Data lake (S3 / GCS / local)",desc:"Pull CSV/JSON from a bucket or path."},
  {kind:"mssql",name:"SQL Server",desc:"Connect by host/IP, user & password; run a query.",api:true},
  {kind:"mongodb",name:"MongoDB",desc:"Connect by URI; pull a collection as records.",api:true},
  {kind:"jira",name:"Jira",desc:"Pull issues via the Jira REST API (base URL + token).",api:true},
  {kind:"powerbi",name:"Power BI",desc:"Pull a dataset/query via the Power BI REST API.",api:true},
  {kind:"looker",name:"Looker",desc:"Pull a Look/query result via the Looker API.",api:true},
  {kind:"webhook",name:"REST / Webhook / API",desc:"Pull JSON records from any REST endpoint.",api:true},
];
function renderConnectorCards(){ const w=$("#connectorCards"); if(!w)return; w.innerHTML="";
  CONNECTORS.forEach(c=>{ const card=el("div","card conn"); card.innerHTML=`<div class="ct">⇄ ${esc(c.name)}</div><div class="cd">${esc(c.desc)}</div>`; card.addEventListener("click",()=>connectorModal(c)); w.appendChild(card); }); }
function connectorModal(c){
  let fields="";
  if(c.kind==="csv"||c.kind==="json"){ fields=`<div class="field">File path<input id="cPath" placeholder="/path/to/data.csv or .json" /></div>`; }
  else if(c.kind==="postgres"||c.kind==="mysql"){ fields=`
    <div class="field">Host / IP<input id="cHost" placeholder="127.0.0.1" /></div>
    <div class="field">Port<input id="cPort" placeholder="${c.kind==="postgres"?"5432":"3306"}" /></div>
    <div class="field">Database<input id="cDb" placeholder="intel" /></div>
    <div class="field">User<input id="cUser" placeholder="analyst" /></div>
    <div class="field">Password<input id="cPass" type="password" placeholder="••••••" /></div>
    <div class="field">Query<textarea id="cQuery" rows="2" placeholder="SELECT * FROM people LIMIT 5000"></textarea></div>`; }
  else if(c.kind==="bigquery"){ fields=`<div class="field">SQL (standard)<textarea id="cQuery" rows="3" placeholder="SELECT * FROM \`proj.dataset.table\` LIMIT 5000"></textarea></div>`; }
  else if(c.kind==="datalake"){ fields=`
    <div class="field">Provider<select id="cProv" class="select"><option value="local">local</option><option value="s3">s3</option><option value="gcs">gcs</option></select></div>
    <div class="field">URI<input id="cUri" placeholder="s3://bucket/export.csv or /path/file.json" /></div>`; }
  else if(c.kind==="mssql"){ fields=`
    <div class="field">Host / IP<input id="cHost" placeholder="127.0.0.1" /></div>
    <div class="field">Port<input id="cPort" placeholder="1433" /></div>
    <div class="field">Database<input id="cDb" placeholder="intel" /></div>
    <div class="field">User<input id="cUser" placeholder="analyst" /></div>
    <div class="field">Password<input id="cPass" type="password" placeholder="••••••" /></div>
    <div class="field">Query<textarea id="cQuery" rows="2" placeholder="SELECT TOP 5000 * FROM people"></textarea></div>`; }
  else if(c.kind==="mongodb"){ fields=`
    <div class="field">Connection URI<input id="cUri" placeholder="mongodb://user:pass@host:27017/intel" /></div>
    <div class="field">Collection<input id="cColl" placeholder="people" /></div>
    <div class="field">Filter (optional JSON)<input id="cQuery" placeholder='{"active":true}' /></div>
    <div class="field">Limit<input id="cLimit" placeholder="5000" /></div>`; }
  else if(c.kind==="jira"){ fields=`
    <div class="field">Endpoint URL<input id="cEndpoint" placeholder="https://org.atlassian.net/rest/api/3/search?jql=project=OPS" /></div>
    <div class="field">Email<input id="cUser" placeholder="you@org.com" /></div>
    <div class="field">API token<input id="cToken" type="password" placeholder="Jira API token" /></div>`; }
  else if(c.kind==="powerbi"||c.kind==="looker"||c.kind==="webhook"){ fields=`
    <div class="field">Endpoint URL<input id="cEndpoint" placeholder="https://api.example.com/v1/data" /></div>
    <div class="field">Bearer token (optional)<input id="cToken" type="password" placeholder="access token" /></div>`; }
  openModal(`Connect: ${c.name}`, fields+`<div class="modal-note" id="connNote"></div>`,[
    {label:"Test",cls:"ghost",act:()=>connectorAction(c,"test")},
    {label:"Import & run",cls:"primary",act:()=>connectorAction(c,"run")},
  ]);
}
function connectorConfig(c){
  if(c.kind==="csv"||c.kind==="json") return {path:$("#cPath").value.trim()};
  if(c.kind==="postgres"||c.kind==="mysql") return {host:$("#cHost").value.trim(),port:$("#cPort").value.trim(),database:$("#cDb").value.trim(),user:$("#cUser").value.trim(),password:$("#cPass").value,query:$("#cQuery").value.trim()};
  if(c.kind==="bigquery") return {query:$("#cQuery").value.trim()};
  if(c.kind==="datalake") return {provider:$("#cProv").value,uri:$("#cUri").value.trim()};
  if(c.kind==="mssql") return {host:$("#cHost").value.trim(),port:$("#cPort").value.trim(),database:$("#cDb").value.trim(),user:$("#cUser").value.trim(),password:$("#cPass").value,query:$("#cQuery").value.trim()};
  if(c.kind==="mongodb") return {uri:$("#cUri").value.trim(),collection:$("#cColl").value.trim(),query:$("#cQuery").value.trim(),limit:$("#cLimit").value.trim()};
  if(c.kind==="jira") return {endpoint:$("#cEndpoint").value.trim(),user:$("#cUser").value.trim(),token:$("#cToken").value};
  if(c.kind==="powerbi"||c.kind==="looker"||c.kind==="webhook") return {endpoint:$("#cEndpoint").value.trim(),token:$("#cToken").value};
  return {};
}
async function connectorAction(c,mode){
  const t=activeTab(); const cfg=connectorConfig(c);
  const note=$("#connNote");
  if(c.kind==="csv"||c.kind==="json"){ // file connectors run the pipeline directly
    if(mode==="test"){ note.textContent="File connector: click Import & run."; return; }
    closeModal();
    const params={inputs:[cfg.path],domain:t?t.project.domain:"generic",provider:state.provider,maxRecords:4000,projectId:t?t.project.id:null};
    setSync("busy","running");
    try{ const result=await runJob("run",params); t.result=result; t.graph=consolidatedToGraph(result); t.project=await api(`/api/projects/get?id=${encodeURIComponent(t.project.id)}`).catch(()=>t.project); setSync("ok","complete"); renderAll(); showView("graph"); setTimeout(()=>{initCy();if(cy)cy.fit(cy.elements(),50);},700); pushNotif("import",`Imported ${cfg.path}`); toast("Imported & processed","ok"); }
    catch(e){ setSync("err","failed"); toast(e.message,"err"); }
    return;
  }
  try {
    if(mode==="test"){ const r=await api("/api/connectors/test",{method:"POST",body:{kind:c.kind,config:cfg}}); note.textContent="✓ "+(r.status||"ok"); if(t) await api("/api/projects",{}).catch(()=>{}); return; }
    closeModal(); setSync("busy","fetching");
    const result=await api("/api/connectors/run",{method:"POST",body:{kind:c.kind,config:cfg,domain:t?t.project.domain:"generic",provider:state.provider,projectId:t?t.project.id:null}});
    t.result=result; t.graph=consolidatedToGraph(result); t.project=await api(`/api/projects/get?id=${encodeURIComponent(t.project.id)}`).catch(()=>t.project);
    setSync("ok","complete"); renderAll(); showView("graph"); setTimeout(()=>{initCy();if(cy)cy.fit(cy.elements(),50);},700);
    pushNotif("connect",`Connected ${c.name}`); toast("Connected & processed","ok");
  } catch(e){ if(note) note.textContent="✗ "+e.message; else toast(e.message,"err"); setSync("err","failed"); }
}
function renderSavedConnectors(){ const t=activeTab(); const w=$("#savedConnectors"); if(!w)return; w.innerHTML="";
  const cs=(t&&t.project.connectors)||[]; if(!cs.length){ w.innerHTML='<div class="empty">—</div>'; return; }
  cs.forEach(c=>{ const li=el("div","li"); li.appendChild(el("span","label",`${c.name} (${c.kind})`)); li.appendChild(el("span","chip",new Date(c.added_at*1000).toISOString().slice(0,10))); w.appendChild(li); }); }
async function refreshDoctor(){ const w=$("#backendList"); if(!w)return; w.innerHTML='<div class="empty">checking…</div>';
  try{ const rows=await api("/api/doctor"); w.innerHTML=""; rows.forEach(r=>{ const li=el("div","li"); li.appendChild(el("span",null,r.name)); const tg=el("span","tag "+(r.ok?"ok":"err"),r.ok?"online":"unavailable"); tg.title=r.detail; li.appendChild(tg); w.appendChild(li); }); }
  catch(e){ w.innerHTML='<div class="empty">'+esc(e.message)+'</div>'; } }
$("#btnDoctor").addEventListener("click",refreshDoctor);

// ---------- plugins ----------
async function renderPlugins(){ const w=$("#pluginList"); if(!w)return; w.innerHTML="";
  let list=[]; try{ list=await api("/api/plugins"); }catch(e){}
  if(!list.length){ w.innerHTML='<div class="empty">No plugins installed.</div>'; return; }
  list.forEach(p=>{ const li=el("div","li"); const l=el("div","l"); l.appendChild(el("span","label",`${p.name} v${p.version||"1"}`)); li.appendChild(l);
    const tg=el("span","tag "+(p.enabled?"ok":"off"),p.enabled?"enabled":"disabled"); tg.style.cursor="pointer"; tg.title="toggle";
    tg.addEventListener("click",async()=>{ try{ await api("/api/plugins/enable",{method:"POST",body:{id:p.id,enabled:!p.enabled}}); renderPlugins(); }catch(e){toast(e.message,"err");} });
    li.appendChild(tg); li.title=p.description||""; w.appendChild(li); }); }
function renderPluginExample(){ const ex=$("#pluginExample"); if(!ex)return;
  ex.textContent = JSON.stringify({ name:"EDU signals", version:"1.0", domains:["generic"],
    field_mappings:[{field:"Student Type",kind:"organization"}], risk_signals:[{token:"suspended",weight:0.7}],
    prompt_addon:"Emphasize enrollment anomalies." }, null, 2); }
$("#btnImportPlugin").addEventListener("click",()=>pickFile(async text=>{ try{ await api("/api/plugins/install",{method:"POST",raw:true,body:text}); renderPlugins(); pushNotif("plugin","Plugin installed"); toast("Plugin installed","ok"); }catch(e){toast(e.message,"err");} }));

// ---------- import/export project ----------
$("#btnExportProject").addEventListener("click",async()=>{ const t=activeTab(); if(!t){toast("Open a project","err");return;}
  try{ const bundle = MODE==="mock" ? JSON.stringify(t.project,null,2) : await (await fetch(`/api/projects/export?id=${encodeURIComponent(t.project.id)}`,{headers:{Authorization:"Bearer "+TOKEN}})).text();
    downloadText(`${t.project.name.replace(/\s+/g,"_")}.cortex.json`, bundle); toast("Exported","ok"); }catch(e){toast(e.message,"err");} });
$("#btnImportProject").addEventListener("click",()=>pickFile(async text=>{ try{ const p=await api("/api/projects/import",{method:"POST",raw:true,body:text}); await loadProjects(); openProject(p.id); toast("Project imported","ok"); }catch(e){toast(e.message,"err");} }));
$("#btnDeleteProject").addEventListener("click",async()=>{ const t=activeTab(); if(!t)return;
  openModal("Delete project?",`<p class="muted">This permanently deletes "${esc(t.project.name)}" and its saved result.</p>`,[
    {label:"Cancel",cls:"ghost",act:closeModal},
    {label:"Delete",cls:"primary",act:async()=>{ try{ await api("/api/projects/delete",{method:"POST",body:{id:t.project.id}}); closeModal(); closeTab(state.active); loadProjects(); toast("Deleted","ok"); }catch(e){toast(e.message,"err");} }}
  ]); });

// ---------- settings ----------
function renderSettings(){ const a=$("#acctInfo"); if(a){ a.innerHTML=""; const u=state.user||{}; [["Name",u.display_name],["Email",u.email],["Role",u.role]].forEach(([k,v])=>{ const r=el("div","row"); r.appendChild(el("span","k",k)); r.appendChild(el("span","v",v||"—")); a.appendChild(r); }); }
  const p=$("#projInfo"); const t=activeTab(); if(p){ p.innerHTML=""; if(t){ [["Name",t.project.name],["Vertical",t.project.domain],["Connectors",String(t.project.connectors.length)],["Activities",String(t.project.activities.length)]].forEach(([k,v])=>{ const r=el("div","row"); r.appendChild(el("span","k",k)); r.appendChild(el("span","v",v)); p.appendChild(r); }); } else p.innerHTML='<div class="empty">No active project.</div>'; } }
$("#btnLogout").addEventListener("click",logout);
// language selector: reflect current, switch on change
(function(){ const sel=$("#setLang"); if(sel){ sel.value=LANG; sel.addEventListener("change",()=>setLang(sel.value)); } })();

// ---------- provider select (custom) ----------
const SELECTS={};
function makeSelect(id,options,value,onChange){ const root=$("#"+id); if(!root)return; root.innerHTML="";
  const btn=el("div","cs-btn"); const lbl=el("span","cs-lbl"); btn.appendChild(lbl); btn.appendChild(el("span","cs-caret","▾")); root.appendChild(btn);
  const list=el("div","cs-list"); root.appendChild(list);
  const api2={value,set(v){this.value=v; const o=options.find(x=>x.value===v)||options[0]; lbl.textContent=o?o.label:""; Array.from(list.children).forEach(c=>c.classList.toggle("sel",c.dataset.v===v));}};
  options.forEach(o=>{ const it=el("div","cs-opt",o.label); it.dataset.v=o.value; it.addEventListener("click",e=>{e.stopPropagation(); api2.set(o.value); root.classList.remove("open"); onChange&&onChange(o.value);}); list.appendChild(it); });
  btn.addEventListener("click",e=>{e.stopPropagation(); const open=root.classList.contains("open"); closeAllSelects(); if(!open)root.classList.add("open");});
  api2.set(value); SELECTS[id]=api2; }
function closeAllSelects(){ $$(".cselect.open").forEach(s=>s.classList.remove("open")); }
function buildProviderSelect(){ const opts=[{value:"auto",label:"Auto — smart routing (Opus/Sonnet ⇄ Codex)"},{value:"claude",label:"Claude (Opus 4.8 / Sonnet)"},{value:"codex",label:"ChatGPT Codex (gpt-5.5)"},{value:"mock",label:"Offline mock"}];
  makeSelect("setProvider",opts,state.provider,v=>{ state.provider=v; $("#providerPill").textContent="provider: "+v; }); }

// ---------- notifications ----------
function pushNotif(kind,text){ state.notifications.unshift({kind,text,at:Date.now()}); const b=$("#notifBadge"); b.hidden=false; b.textContent=state.notifications.length; renderNotifs(); }
function renderNotifs(){ const w=$("#notifList"); if(!w)return; w.innerHTML=""; if(!state.notifications.length){ w.innerHTML='<div class="empty">No notifications.</div>'; return; }
  state.notifications.slice(0,50).forEach(n=>{ const li=el("div","li"); li.appendChild(el("span","label",n.text)); li.appendChild(el("span","chip",n.kind)); w.appendChild(li); }); }
$("#btnNotifications").addEventListener("click",()=>{ const d=$("#notifDrawer"); d.hidden=!d.hidden; $("#notifBadge").hidden=true; });
$("#notifClose").addEventListener("click",()=>$("#notifDrawer").hidden=true);

// ---------- modal ----------
function openModal(title,bodyHtml,buttons){ $("#modalTitle").textContent=title; $("#modalBody").innerHTML=bodyHtml;
  const foot=$("#modalFoot"); foot.innerHTML=""; (buttons||[]).forEach(b=>{ const btn=el("button","btn "+(b.cls||"ghost"),b.label); btn.addEventListener("click",b.act); foot.appendChild(btn); });
  $("#modalBackdrop").hidden=false; }
function closeModal(){ $("#modalBackdrop").hidden=true; closeAllSelects(); }
$("#modalClose").addEventListener("click",closeModal);
$("#modalBackdrop").addEventListener("click",e=>{ if(e.target===$("#modalBackdrop")) closeModal(); });
$("#btnNewProject").addEventListener("click",newProjectModal);
$("#btnNewProject2").addEventListener("click",newProjectModal);

// ---------- graph controls ----------
$("#btnFit").addEventListener("click",()=>{ if(cy) cy.fit(cy.elements(":visible"),50); });
$("#zoomIn")&&$("#zoomIn").addEventListener("click",()=>{ if(cy) cy.zoom({level:cy.zoom()*1.3, renderedPosition:{x:cy.width()/2,y:cy.height()/2}}); });
$("#zoomOut")&&$("#zoomOut").addEventListener("click",()=>{ if(cy) cy.zoom({level:cy.zoom()/1.3, renderedPosition:{x:cy.width()/2,y:cy.height()/2}}); });
$("#zoomFit")&&$("#zoomFit").addEventListener("click",()=>{ if(cy) cy.fit(cy.elements(":visible"),50); });
$("#zoomIn")&&($("#zoomIn").innerHTML=svg("zoomin")); $("#zoomOut")&&($("#zoomOut").innerHTML=svg("zoomout")); $("#zoomFit")&&($("#zoomFit").innerHTML=svg("fit"));

// ---------- minimap ----------
let _mmScheduled=false;
function scheduleMinimap(){ if(_mmScheduled)return; _mmScheduled=true; requestAnimationFrame(()=>{ _mmScheduled=false; drawMinimap(); }); }
function drawMinimap(){ const cv=$("#minimap"); if(!cv) return; const ctx=cv.getContext("2d");
  if(!cy||!cy.nodes(":visible").length){ ctx.clearRect(0,0,cv.width,cv.height); return; }
  const dpr=window.devicePixelRatio||1; const W=cv.clientWidth||190, H=cv.clientHeight||130;
  if(cv.width!==W*dpr||cv.height!==H*dpr){ cv.width=W*dpr; cv.height=H*dpr; }
  ctx.setTransform(dpr,0,0,dpr,0,0); ctx.clearRect(0,0,W,H);
  const bb=cy.elements(":visible").boundingBox(); const pad=8, sw=W-2*pad, sh=H-2*pad;
  const s=Math.min(sw/(bb.w||1), sh/(bb.h||1)); const offx=pad+(sw-bb.w*s)/2, offy=pad+(sh-bb.h*s)/2;
  const X=x=>offx+(x-bb.x1)*s, Y=y=>offy+(y-bb.y1)*s;
  ctx.strokeStyle="rgba(120,140,165,0.14)"; ctx.lineWidth=0.4; ctx.beginPath();
  cy.edges(":visible").forEach(e=>{ const a=e.source().position(), b=e.target().position(); ctx.moveTo(X(a.x),Y(a.y)); ctx.lineTo(X(b.x),Y(b.y)); }); ctx.stroke();
  cy.nodes(":visible").forEach(n=>{ const p=n.position(); const nd=nodeData(n.id()); ctx.fillStyle=nd?kColor(nd.kind):"#889"; ctx.beginPath(); ctx.arc(X(p.x),Y(p.y),1.4,0,7); ctx.fill(); });
  const ext=cy.extent(); ctx.fillStyle="rgba(51,194,221,0.09)"; ctx.strokeStyle="rgba(51,194,221,0.9)"; ctx.lineWidth=1;
  const rx=X(ext.x1), ry=Y(ext.y1), rw=(ext.x2-ext.x1)*s, rh=(ext.y2-ext.y1)*s; ctx.fillRect(rx,ry,rw,rh); ctx.strokeRect(rx,ry,rw,rh);
  cv._map={bb,s,offx,offy};
}
function minimapGoto(ev){ const cv=$("#minimap"); const m=cv._map; if(!m||!cy)return; const r=cv.getBoundingClientRect();
  const mx=ev.clientX-r.left, my=ev.clientY-r.top; const modelX=m.bb.x1+(mx-m.offx)/m.s, modelY=m.bb.y1+(my-m.offy)/m.s;
  const z=cy.zoom(); cy.pan({x:cy.width()/2-modelX*z, y:cy.height()/2-modelY*z}); }
(function(){ const cv=$("#minimap"); if(!cv)return; let down=false;
  cv.addEventListener("mousedown",e=>{down=true;minimapGoto(e);});
  window.addEventListener("mousemove",e=>{ if(down)minimapGoto(e); });
  window.addEventListener("mouseup",()=>down=false); })();

// ---------- path finder ----------
let pathSource=null;
function pathBanner(txt){ let b=$("#pathBanner"); if(txt){ if(!b){ b=el("div","pathbar"); b.id="pathBanner"; $(".graph-wrap").appendChild(b);} b.textContent=txt; b.hidden=false; } else if(b) b.hidden=true; }
function startPath(id){ pathSource=id; if(cy) cy.elements().removeClass("pathhl faded"); pathBanner("Path mode: click the target node (Esc to cancel)"); }
function finishPath(targetId){ const src=pathSource; pathSource=null; pathBanner(""); if(!cy||src===targetId) return;
  const res=cy.elements().aStar({root:cy.$id(src), goal:cy.$id(targetId), directed:false});
  if(!res.found){ toast("No path between these entities","err"); return; }
  cy.elements().addClass("faded"); res.path.removeClass("faded").addClass("pathhl");
  cy.fit(res.path,80);
  const nodes=res.path.nodes().map(n=>{ const d=nodeData(n.id()); return d?d.label:n.id(); });
  toast(`Path found · ${res.path.nodes().length} hops`,"ok");
  // show the path chain in the dossier relations area
  const c=$("#context"); c.hidden=false; $("#ctxKind").textContent="path"; $("#ctxName").textContent="Path trace";
  $("#ctxRisk").innerHTML=""; $("#ctxTags").innerHTML=""; $("#ctxMeta").innerHTML=""; $("#ctxSources").innerHTML=""; $("#ctxTransforms").innerHTML="";
  const rl=$("#ctxRels"); rl.innerHTML=""; nodes.forEach((lab,i)=>{ const r=el("div","rel"); r.innerHTML=(i<nodes.length-1?`<span class="rt">hop ${i+1}</span> `:`<span class="rt">end</span> `)+esc(lab); rl.appendChild(r); });
}
function clearPath(){ pathSource=null; pathBanner(""); if(cy) cy.elements().removeClass("pathhl faded"); }
$("#btnPath")&&$("#btnPath").addEventListener("click",()=>{ const sel=cy&&cy.$(":selected").length?cy.$(":selected")[0].id():null; if(sel){ startPath(sel); } else { toast("Select a node, then click Path — or right-click a node → Find path from here"); } });

// ---------- add entity manually (incl. media for analysis) ----------
let aeUploadPath=null;
function addEntityModal(){ const t=activeTab(); if(!t){ toast("Open or create a project first","err"); return; }
  const kinds=["person","account","organization","ip","domain","url","media","evidence","device","wallet","payment","group","location","communication","malware","incident","vulnerability","suspect","victim","case","report","service","repository"];
  const kopts=kinds.map(k=>`<option value="${k}">${k}</option>`).join("");
  openModal("Add entity", `
    <div class="field">Type<select id="aeKind" class="select">${kopts}</select></div>
    <div class="field">Label / value<input id="aeLabel" placeholder="name, email, IP, domain, file name…" /></div>
    <div class="field" id="aeMediaField" hidden>Media file (image / video / audio) — uploaded for metadata & authenticity analysis
      <div style="display:flex;gap:8px"><input id="aeFile" placeholder="no file selected" readonly style="flex:1"/><button class="btn ghost" id="aeBrowse">Browse…</button></div>
      <select id="aeMediaType" class="select" style="margin-top:8px"><option value="image">image</option><option value="video">video</option><option value="audio">audio</option><option value="document">document</option></select>
      <div class="disclaimer" style="margin-top:8px">Media is referenced by path/hash. Sensitive material is gated; run the media transforms (metadata, deepfake, sensitive-content) to analyze.</div>
    </div>
    <div class="field">Attributes (key: value per line, optional)<textarea id="aeAttrs" rows="2" placeholder="source: hotline&#10;country: BR"></textarea></div>
  `,[
    {label:"Cancel",cls:"ghost",act:closeModal},
    {label:"Add entity",cls:"primary",act:doAddEntity}
  ]);
  aeUploadPath=null;
  setTimeout(()=>{ const ks=$("#aeKind"); const upd=()=>{ $("#aeMediaField").hidden=!["media","evidence"].includes(ks.value); }; ks&&ks.addEventListener("change",upd); upd();
    const b=$("#aeBrowse"); if(b)b.addEventListener("click",()=>pickServerPath(p=>{ aeUploadPath=p; $("#aeFile").value=p.split("/").pop(); if(!$("#aeLabel").value) $("#aeLabel").value=p.split("/").pop(); }, {title:"Choose media file", accept:".png,.jpg,.jpeg,.gif,.webp,.mp4,.mov,.avi,.mp3,.wav,.m4a,.pdf"})); },40);
}
function doAddEntity(){ const t=activeTab(); if(!t)return; const kind=$("#aeKind").value; let label=$("#aeLabel").value.trim();
  if(!label && !aeUploadPath){ toast("Label or file required","err"); return; }
  const attrs={}; ($("#aeAttrs").value||"").split("\n").forEach(l=>{ const i=l.indexOf(":"); if(i>0){ const k=l.slice(0,i).trim(); if(k)attrs[k]=l.slice(i+1).trim(); } });
  if(["media","evidence"].includes(kind)){ attrs.media_type=$("#aeMediaType").value; if(aeUploadPath){ attrs.path=aeUploadPath; attrs.file=aeUploadPath.split("/").pop(); if(!label)label=aeUploadPath.split("/").pop(); } }
  if(!label) label=kind+" (manual)";
  const id="man-"+Math.abs(hashStr(kind+label+String(state.tabs.length)+Object.keys(attrs).join()));
  t.graph.nodes.push({ id, kind, label, risk:0.3, band:"low", attributes:attrs, tags:["manual"], sources:["manual"], sensitive:["media","evidence","victim","communication"].includes(kind) });
  closeModal(); renderGraph(); renderGraphFilters(); showView("graph"); setTimeout(()=>{ selectNode(id); if(cy){const e=cy.$id(id); if(e){e.addClass("fresh"); setTimeout(()=>e.removeClass("fresh"),1800);} } },250);
  pushNotif("entity","Manual entity added: "+label); toast("Entity added — run transforms to analyze","ok");
}
$("#btnAddEntity")&&$("#btnAddEntity").addEventListener("click",addEntityModal);

// ---------- edit an existing entity (analyst correction / change media) ----------
let eeUploadPath=null;
function editEntityModal(id){ const t=activeTab(); if(!t)return; const n=t.graph.nodes.find(x=>x.id===id);
  if(!n){ toast("Entity not found","err"); return; }
  const isMedia=["media","evidence"].includes(n.kind);
  const attrsText=Object.entries(n.attributes||{}).filter(([k])=>!["path","file"].includes(k)).map(([k,v])=>`${k}: ${v}`).join("\n");
  const curFile=(n.attributes&&(n.attributes.file||n.attributes.path))||"";
  eeUploadPath=null;
  openModal("Edit entity", `
    <div class="field">Type<input value="${esc(n.kind)}" disabled /></div>
    <div class="field">Label / value<input id="eeLabel" value="${esc(n.label||"")}" /></div>
    <div class="field">Tags (comma-separated)<input id="eeTags" value="${esc((n.tags||[]).join(", "))}" /></div>
    ${isMedia?`<div class="field">Media file
      <div style="display:flex;gap:8px"><input id="eeFile" readonly style="flex:1" value="${esc(curFile.split("/").pop())}" placeholder="no file selected"/><button class="btn ghost" id="eeBrowse">Change…</button></div>
      <div class="modal-note">Pick a different image/video/audio to replace the referenced media.</div></div>`:""}
    <div class="field">Attributes (key: value per line)<textarea id="eeAttrs" rows="3">${esc(attrsText)}</textarea></div>
  `,[{label:"Cancel",cls:"ghost",act:closeModal},{label:"Save changes",cls:"primary",act:()=>saveEntityEdit(id)}]);
  if(isMedia) setTimeout(()=>{ const b=$("#eeBrowse"); if(b)b.addEventListener("click",()=>pickServerPath(p=>{ eeUploadPath=p; $("#eeFile").value=p.split("/").pop(); }, {title:"Choose replacement media", accept:".png,.jpg,.jpeg,.gif,.webp,.mp4,.mov,.avi,.mp3,.wav,.m4a,.pdf"})); },40);
  setTimeout(()=>$("#eeLabel")&&$("#eeLabel").focus(),50);
}
function saveEntityEdit(id){ const t=activeTab(); if(!t)return; const n=t.graph.nodes.find(x=>x.id===id); if(!n)return;
  const label=$("#eeLabel").value.trim(); if(label) n.label=label;
  n.tags=($("#eeTags").value||"").split(",").map(s=>s.trim()).filter(Boolean);
  const attrs={}; ($("#eeAttrs").value||"").split("\n").forEach(l=>{ const i=l.indexOf(":"); if(i>0){ const k=l.slice(0,i).trim(); if(k)attrs[k]=l.slice(i+1).trim(); } });
  // preserve media path/file unless a new file was chosen
  if(n.attributes){ if(n.attributes.media_type&&!attrs.media_type) attrs.media_type=n.attributes.media_type; }
  if(eeUploadPath){ attrs.path=eeUploadPath; attrs.file=eeUploadPath.split("/").pop(); }
  else if(n.attributes&&n.attributes.path){ attrs.path=n.attributes.path; attrs.file=n.attributes.file; }
  n.attributes=attrs;
  if(!n.tags.includes("edited")) n.tags.push("edited");
  closeModal(); renderGraph(); renderGraphFilters(); selectNode(id); toast("Entity updated","ok");
}
$("#btnAddEntity2")&&$("#btnAddEntity2").addEventListener("click",addEntityModal);
$("#btnReset").addEventListener("click",()=>{ if(cy){ cy.elements().style("display","element").removeClass("faded pathhl"); cy.$(":selected").unselect(); cy.fit(cy.elements(),50); } $("#graphFilter").value=""; $("#context").hidden=true; clearTimeScrub(); modeHint(""); const t=activeTab(); if(t){ t.graphMode="full"; t.clusterMode="none"; $("#graphCluster")&&($("#graphCluster").value="none"); syncModeButtons(); $("#graphStats").textContent=`${t.graph.nodes.length} nodes · ${t.graph.edges.length} edges`; } toast("View reset"); });
$("#graphLayout").addEventListener("change",runLayout);
$("#graphCluster")&&$("#graphCluster").addEventListener("change",e=>{ const t=activeTab(); if(t)t.graphMode="custom"; syncModeButtons(); setClusterMode(e.target.value); });

// ---------- progressive view modes ----------
// Reveal information on demand so 10k+ node graphs stay legible.
const LARGE_GRAPH = 800;
function modeHint(txt){ let h=$("#modeHint"); if(txt){ if(!h){ h=el("div","mode-hint"); h.id="modeHint"; $(".graph-wrap").appendChild(h);} h.innerHTML=txt; h.hidden=false; } else if(h) h.hidden=true; }
function syncModeButtons(){ const t=activeTab(); const m=t?(t.graphMode||"full"):"full"; $$(".gmode").forEach(b=>b.classList.toggle("active",b.dataset.mode===m)); }
function parseTs(n){ for(const k of ["created_at","created at","timestamp","first_seen_at","observed_at","received_at","date"]){ const v=(n.attributes||{})[k]; if(v){ const d=Date.parse(v); if(!isNaN(d))return d; } } return null; }
function clearTimeScrub(){ const s=$("#timeScrub"); if(s)s.remove(); }
function setGraphMode(mode, opts){ const t=activeTab(); if(!t||!t.graph.nodes.length){ toast("No graph to view","err"); return; }
  t.graphMode=mode; syncModeButtons(); clearTimeScrub(); modeHint("");
  showView("graph");
  // Overview clusters; every other mode needs the un-clustered graph. If we're
  // leaving a clustered state, rebuild the individual nodes first.
  const wasClustered=(t.clusterMode||"none")!=="none";
  if(mode==="overview"){ t.clusterMode="none"; setClusterMode("kind"); $("#graphCluster")&&($("#graphCluster").value="kind"); modeHint(`<b>Overview</b> — entities grouped by type · double-click a cluster to drill in`); syncModeButtons(); return; }
  t.clusterMode="none"; $("#graphCluster")&&($("#graphCluster").value="none");
  if(wasClustered) renderGraph(); // rebuild individual nodes into cy
  const delay = wasClustered ? 350 : 0;
  setTimeout(()=>requestAnimationFrame(()=>{ initCy(); if(!cy)return; cy.resize(); cy.elements().style("display","element").removeClass("faded");
    if(mode==="full"){ modeHint(`<b>Full</b> — all ${t.graph.nodes.length} entities`); cy.fit(cy.elements(),50); }
    else if(mode==="risk"){ const keep=new Set(); t.graph.nodes.forEach(n=>{ const b=n.band||bandOf(n.risk); if(b==="high"||b==="critical")keep.add(n.id); });
      t.graph.edges.forEach(e=>{ if(keep.has(e.source))keep.add(e.target); if(keep.has(e.target))keep.add(e.source); });
      cy.nodes().forEach(n=>n.style("display",keep.has(n.id())?"element":"none")); cy.edges().forEach(ed=>ed.style("display",(keep.has(ed.source().id())&&keep.has(ed.target().id()))?"element":"none"));
      if(keep.size) cy.fit(cy.nodes(":visible"),60); modeHint(keep.size?`<b>Risk lens</b> — ${keep.size} high/critical entities and their neighbors`:`<b>Risk lens</b> — no high/critical entities in this graph`); }
    else if(mode==="neighborhood"){ let seedId=(opts&&opts.seed)|| (cy.$(":selected").length?cy.$(":selected")[0].id():null) || [...t.graph.nodes].sort((a,b)=>b.risk-a.risk)[0].id;
      const hops=(opts&&opts.hops)||2; const seed=cy.$id(seedId); if(!seed.length){ modeHint("Select an entity for Neighborhood"); return; }
      let nb=seed.closedNeighborhood(); for(let i=1;i<hops;i++) nb=nb.closedNeighborhood();
      cy.elements().style("display","none"); nb.style("display","element"); cy.fit(nb,70); selectNode(seedId); t._nbSeed=seedId;
      modeHint(`<b>Neighborhood</b> — ${hops} hop(s) around "${nodeData(seedId).label}" · right-click another node → Neighborhood`); }
    else if(mode==="timeline"){ buildTimeline(t); }
  }), delay);
}
function buildTimeline(t){ const withTs=t.graph.nodes.map(n=>({n,ts:parseTs(n)})).filter(x=>x.ts!=null);
  if(withTs.length<2){ modeHint(`<b>Timeline</b> — not enough timestamped entities (need created_at/timestamp attributes)`); cy.fit(cy.elements(),50); return; }
  const times=withTs.map(x=>x.ts).sort((a,b)=>a-b); const min=times[0], max=times[times.length-1];
  const fmt=d=>new Date(d).toISOString().slice(0,10);
  const scrub=el("div","time-scrub"); scrub.id="timeScrub";
  scrub.innerHTML=`<span class="ts-label" id="tsLabel">${fmt(min)} → cutoff</span><input type="range" id="tsRange" min="${min}" max="${max}" value="${max}" step="${Math.max(1,Math.floor((max-min)/200))}"><span class="ts-label" id="tsMax" style="text-align:right">${fmt(max)}</span>`;
  $(".graph-wrap").appendChild(scrub);
  const apply=cutoff=>{ const keep=new Set(); t.graph.nodes.forEach(n=>{ const ts=parseTs(n); if(ts==null||ts<=cutoff)keep.add(n.id); });
    cy.nodes().forEach(n=>n.style("display",keep.has(n.id())?"element":"none")); cy.edges().forEach(ed=>ed.style("display",(keep.has(ed.source().id())&&keep.has(ed.target().id()))?"element":"none"));
    $("#tsLabel").textContent=`up to ${fmt(cutoff)}`; };
  $("#tsRange").addEventListener("input",e=>apply(parseInt(e.target.value)));
  apply(max); cy.fit(cy.nodes(":visible"),50);
  modeHint(`<b>Timeline</b> — drag the slider to reveal entities up to a point in time (${withTs.length} timestamped)`);
}
$$(".gmode").forEach(b=>b.addEventListener("click",()=>setGraphMode(b.dataset.mode)));
$("#graphFilter").addEventListener("input",e=>{ const q=e.target.value.trim().toLowerCase(); if(!cy)return;
  if(!q){ cy.nodes().style("display","element"); } else { cy.nodes().forEach(n=>{ const nd=nodeData(n.id()); const show=nd&&(nd.label+" "+nd.kind).toLowerCase().includes(q); n.style("display",show?"element":"none"); }); }
});
$("#globalSearch").addEventListener("keydown",e=>{ if(e.key==="Enter"){ const q=e.target.value.trim().toLowerCase(); const t=activeTab(); const hit=t&&t.graph.nodes.find(n=>n.label.toLowerCase().includes(q)); if(hit) selectNode(hit.id); else toast("No match"); } });

// ---------- command palette ----------
const COMMANDS=[
  ["New project","⌘N",newProjectModal],["Run analysis","⌘R",runModal],["Add entity","",addEntityModal],["Ask AI copilot","⌘/",openAsk],
  ["Generate intelligence","",()=>{showView("intelligence");renderIntelligence();generateIntelligence();}],
  ["Go to Dashboard","",()=>showView("dashboard")],["Go to Graph","",()=>showView("graph")],["Go to Intelligence","",()=>{showView("intelligence");renderIntelligence();}],["Go to Entities","",()=>showView("entities")],
  ["Go to Timeline","",()=>showView("timeline")],["Go to Reports","",()=>showView("reports")],
  ["Data Sources","",()=>openSettingsTab("datasources")],["Transforms store","",()=>openSettingsTab("transforms")],["API Keys","",()=>openSettingsTab("keys")],["Settings","",()=>openSettingsTab("account")],
  ["Fit graph","",()=>{showView("graph");if(cy)cy.fit(cy.elements(":visible"),50);}],["Recheck backends","",refreshDoctor],["Sign out","",logout],
];
let palSel=0;
function openPalette(){ $("#paletteBackdrop").hidden=false; $("#paletteInput").value=""; renderPalette(""); $("#paletteInput").focus(); }
function closePalette(){ $("#paletteBackdrop").hidden=true; }
function renderPalette(q){ const list=$("#paletteList"); list.innerHTML=""; palSel=0; const items=COMMANDS.filter(c=>c[0].toLowerCase().includes(q.toLowerCase()));
  items.forEach((c,i)=>{ const d=el("div","palette-item"+(i===0?" sel":"")); d.appendChild(el("span",null,c[0])); d.appendChild(el("span","hint",c[1])); d.addEventListener("click",()=>{c[2]();closePalette();}); list.appendChild(d); }); list._items=items; }
$("#paletteInput").addEventListener("input",e=>renderPalette(e.target.value));
$("#paletteInput").addEventListener("keydown",e=>{ const items=$("#paletteList")._items||[];
  if(e.key==="ArrowDown")palSel=Math.min(items.length-1,palSel+1); else if(e.key==="ArrowUp")palSel=Math.max(0,palSel-1);
  else if(e.key==="Enter"){ if(items[palSel]){items[palSel][2]();closePalette();} return; } else return;
  $$(".palette-item").forEach((d,i)=>d.classList.toggle("sel",i===palSel)); e.preventDefault(); });
$("#paletteBackdrop").addEventListener("click",e=>{ if(e.target===$("#paletteBackdrop")) closePalette(); });

// nav hooks that need lazy render
$$('.nav li').forEach(li=>li.addEventListener("click",()=>{ if(li.dataset.view==="settings")openSettingsTab(currentSettingsTab); if(li.dataset.view==="intelligence")renderIntelligence(); if(li.dataset.view==="entities")renderEntities(); if(li.dataset.view==="reports"){renderReport();renderReports();} }));
$("#profileBtn").addEventListener("click",()=>{showView("settings");openSettingsTab("account");});

// ---------- settings tabs ----------
let currentSettingsTab="account";
function openSettingsTab(tab){ currentSettingsTab=tab; showView("settings");
  $$(".snav").forEach(b=>b.classList.toggle("active",b.dataset.tab===tab));
  $$(".stab").forEach(s=>s.hidden = s.id!=="stab-"+tab);
  if(tab==="account"||tab==="project") renderSettings();
  if(tab==="providers") { /* provider select already built */ }
  if(tab==="datasources"){ renderConnectorCards(); renderSavedConnectors(); refreshDoctor(); }
  if(tab==="transforms"){ renderTransformStore(); renderInstalledTransforms(); }
  if(tab==="keys") renderKeys();
  if(tab==="plugins"){ renderPlugins(); renderPluginExample(); }
  if(tab==="users") renderUsers();
}
$$(".snav").forEach(b=>b.addEventListener("click",()=>openSettingsTab(b.dataset.tab)));

// ---------- RBAC: users & access (admin only) ----------
async function renderUsers(){ const w=$("#usersList"); if(!w)return; const isAdmin=(state.user&&state.user.role==="admin");
  $("#btnAddUser")&&($("#btnAddUser").style.display=isAdmin?"":"none");
  if(!isAdmin){ w.innerHTML='<div class="empty">Only administrators can manage users.</div>'; return; }
  let users=[]; try{ users=await api("/api/users"); }catch(e){ w.innerHTML='<div class="empty">'+esc(e.message)+'</div>'; return; }
  w.innerHTML="";
  users.forEach(u=>{ const li=el("div","li"); const l=el("div","l"); l.appendChild(el("span","label",`${u.display_name} · ${u.email}`)); li.appendChild(l);
    const roleSel=el("select","mini-select"); ["admin","analyst","viewer"].forEach(r=>{ const o=el("option",null,r); o.value=r; if(u.role===r)o.selected=true; roleSel.appendChild(o); });
    roleSel.addEventListener("change",async()=>{ try{ await api("/api/users/role",{method:"POST",body:{id:u.id,role:roleSel.value}}); toast("Role updated","ok"); }catch(e){ toast(e.message,"err"); renderUsers(); } });
    const wrap=el("div"); wrap.style.display="flex"; wrap.style.gap="8px"; wrap.style.alignItems="center"; if(u.locked)wrap.appendChild(el("span","tag err","locked")); wrap.appendChild(roleSel); li.appendChild(wrap); w.appendChild(li); });
}
$("#btnAddUser")&&$("#btnAddUser").addEventListener("click",()=>{
  openModal("Add user", `
    <div class="field">Display name<input id="uName" placeholder="Full name" /></div>
    <div class="field">Email<input id="uEmail" type="email" placeholder="user@org.com" /></div>
    <div class="field">Temporary password<input id="uPass" type="text" placeholder="min 10 chars, upper+lower+digit" /></div>
    <div class="field">Role<select id="uRole" class="select"><option value="analyst">analyst — run & edit</option><option value="viewer">viewer — read-only</option><option value="admin">admin — full control</option></select></div>
  `,[{label:"Cancel",cls:"ghost",act:closeModal},{label:"Create user",cls:"primary",act:async()=>{
    try{ await api("/api/users",{method:"POST",body:{email:$("#uEmail").value,display_name:$("#uName").value,password:$("#uPass").value,role:$("#uRole").value}}); closeModal(); renderUsers(); toast("User created","ok"); }catch(e){ toast(e.message,"err"); } }}]);
  setTimeout(()=>$("#uName")&&$("#uName").focus(),40);
});

// ---------- transform store ----------
const TF_CATS=[["people","People Search"],["kyc","KYC / Identity (BR·US)"],["cyber","Cybersecurity"],["investigative","Investigative / OSINT"],["media","Media Forensics"],["journalism","Journalism"],["hr","Human Resources"],["business","Business & Corporate"],["military","Military Intelligence"]];
async function renderTransformStore(){ const w=$("#transformCatalog"); if(!w)return; w.innerHTML="checking…";
  let cat=[],inst=[]; try{ cat=await api("/api/transforms/catalog"); }catch(e){} try{ inst=await api("/api/transforms"); }catch(e){}
  const instIds=new Set(inst.map(t=>t.id));
  const q=($("#transformSearch")?.value||"").toLowerCase();
  w.innerHTML="";
  TF_CATS.forEach(([slug,title])=>{ const items=cat.filter(t=>t.category===slug && (t.name+t.description).toLowerCase().includes(q)); if(!items.length)return;
    const box=el("div","tf-cat"); box.appendChild(el("h4",null,title));
    items.forEach(t=>{ const it=el("div","tf-item");
      const meta=el("div","tf-meta"); const nm=el("div","tf-name"); nm.appendChild(document.createTextNode(t.name));
      nm.appendChild(el("span","tf-badge "+(t.runtime==="rust"?"rs":"py"), t.runtime));
      nm.appendChild(el("span","tf-badge "+(t.requires_api_key?"key":"free"), t.requires_api_key?("key: "+t.service):"no key"));
      meta.appendChild(nm); meta.appendChild(el("div","tf-desc",t.description)); it.appendChild(meta);
      const btn=el("button","btn "+(instIds.has(t.id)?"ghost":"primary"), instIds.has(t.id)?"Installed":"Install");
      if(!instIds.has(t.id)) btn.addEventListener("click",()=>{ const doInstall=async()=>{ try{ await api("/api/transforms/install",{method:"POST",body:{id:t.id}}); toast("Installed "+t.name,"ok"); renderTransformStore(); renderInstalledTransforms(); if(t.requires_api_key) openSettingsTab("keys"); }catch(e){toast(e.message,"err");} };
        if(t.disclaimer){ openModal("Install "+t.name, `<div class="disclaimer">${esc(t.disclaimer)}</div><p class="muted">${esc(t.description)}</p>`,[{label:"Cancel",cls:"ghost",act:closeModal},{label:"I understand — install",cls:"primary",act:()=>{closeModal();doInstall();}}]); }
        else doInstall(); });
      it.appendChild(btn); box.appendChild(it); });
    w.appendChild(box); });
  if(!w.children.length) w.innerHTML='<div class="empty">No matches.</div>';
}
$("#transformSearch")&&$("#transformSearch").addEventListener("input",renderTransformStore);
async function renderInstalledTransforms(){ const w=$("#installedTransforms"); if(!w)return; let inst=[]; try{ inst=await api("/api/transforms"); }catch(e){}
  w.innerHTML=""; if(!inst.length){ w.innerHTML='<div class="empty">None installed. Add from the store above.</div>'; return; }
  inst.forEach(t=>{ const li=el("div","li"); const l=el("div","l"); l.appendChild(el("span","label",`${t.name} · ${t.category}`)); li.appendChild(l);
    const en=el("span","tag "+(t.enabled?"ok":"off"),t.enabled?"on":"off"); en.style.cursor="pointer"; en.addEventListener("click",async()=>{ await api("/api/transforms/enable",{method:"POST",body:{id:t.id,enabled:!t.enabled}}); renderInstalledTransforms(); });
    const rm=el("span","tag off","remove"); rm.style.cursor="pointer"; rm.style.marginLeft="6px"; rm.addEventListener("click",async()=>{ await api("/api/transforms/remove",{method:"POST",body:{id:t.id}}); renderInstalledTransforms(); renderTransformStore(); });
    const wrap=el("div"); wrap.appendChild(en); wrap.appendChild(rm); li.appendChild(wrap); w.appendChild(li); });
}

// ---------- API keys ----------
async function renderKeys(){ const w=$("#keyList"); if(!w)return; let names=[]; try{ names=await api("/api/keys"); }catch(e){}
  w.innerHTML=""; if(!names.length){ w.innerHTML='<div class="empty">No keys stored.</div>'; return; }
  names.forEach(n=>{ const li=el("div","li"); li.appendChild(el("span","label",n)); const rm=el("span","tag off","delete"); rm.style.cursor="pointer"; rm.addEventListener("click",async()=>{ await api("/api/keys/delete",{method:"POST",body:{service:n}}); renderKeys(); }); li.appendChild(rm); w.appendChild(li); });
}
$("#btnSaveKey")&&$("#btnSaveKey").addEventListener("click",async()=>{ const s=$("#keyService").value.trim(),k=$("#keyValue").value; if(!s||!k){toast("Service and key required","err");return;} try{ await api("/api/keys",{method:"POST",body:{service:s,key:k}}); $("#keyValue").value=""; toast("Key saved","ok"); renderKeys(); }catch(e){toast(e.message,"err");} });

// ---------- graph SIEM filters ----------
let gfActiveKinds=null; // Set or null(all)
let gfPreset="all";
function renderGraphFilters(){ const t=activeTab(); const kinds=[...new Set((t?.graph.nodes||[]).map(n=>n.kind))]; const w=$("#gfKinds"); if(!w)return; w.innerHTML="";
  kinds.forEach(k=>{ const chip=el("div","gf-kind"+((gfActiveKinds&&!gfActiveKinds.has(k))?" off":"")); const d=el("span","kdot"); d.style.background=kColor(k); chip.appendChild(d); chip.appendChild(el("span",null,k));
    chip.addEventListener("click",()=>{ if(!gfActiveKinds) gfActiveKinds=new Set(kinds); if(gfActiveKinds.has(k))gfActiveKinds.delete(k); else gfActiveKinds.add(k); applyFilters(); renderGraphFilters(); }); w.appendChild(chip); });
}
function applyFilters(){ if(!cy)return; const t=activeTab(); if(!t)return;
  cy.nodes().forEach(node=>{ const n=nodeData(node.id()); if(!n){node.style("display","none");return;}
    let ok=true; const band=n.band||bandOf(n.risk);
    if(gfPreset==="crit") ok = ok && (band==="critical"||band==="high");
    else if(gfPreset==="suspicious"){ const hay=(n.tags.join(" ")+" "+n.kind+" "+Object.values(n.attributes||{}).join(" ")).toLowerCase(); ok = ok && (/suspic|malicious|malware|threat|fraud|c2|exploit/.test(hay) || band==="critical" || ["suspect","malware","incident"].includes(n.kind)); }
    else if(gfPreset==="sensitive") ok = ok && !!n.sensitive;
    if(gfActiveKinds && !gfActiveKinds.has(n.kind)) ok=false;
    node.style("display", ok?"element":"none"); });
  cy.fit(cy.nodes(":visible"),50);
}
$$(".gf-preset").forEach(b=>b.addEventListener("click",()=>{ gfPreset=b.dataset.preset; $$(".gf-preset").forEach(x=>x.classList.toggle("active",x===b)); applyFilters(); }));

// ---------- run transforms from entity panel ----------
async function renderCtxTransforms(kind){ const w=$("#ctxTransforms"); if(!w)return; w.innerHTML='<div class="empty">loading…</div>';
  let inst=[]; try{ inst=await api("/api/transforms"); }catch(e){}
  const match=inst.filter(t=>t.enabled && (!t.input_kinds.length || t.input_kinds.includes(kind)));
  w.innerHTML=""; if(!match.length){ w.innerHTML='<div class="empty">No transforms for this kind. Install from Settings → Transforms.</div>'; return; }
  match.forEach(t=>{ const r=el("div","rel"); r.innerHTML=`<span class="rt">${t.runtime}</span> ${esc(t.name)}`; r.addEventListener("click",()=>runTransformOnSelected(t)); w.appendChild(r); });
}
async function runTransformOnSelected(t){ const id=cy&&cy.$(":selected").length?cy.$(":selected")[0].id():null; const n=id?nodeData(id):null; if(!n){toast("Select an entity","err");return;}
  toast("Running "+t.name+"…"); setSync("busy","transform");
  try{ const res=await api("/api/transforms/run",{method:"POST",body:{id:t.id,input:{kind:n.kind,label:n.label,attributes:n.attributes}}});
    if(res.error){ toast("Transform: "+res.error,"err"); setSync("err","failed"); return; }
    mergeTransformResult(n, res); setSync("ok","complete"); pushNotif("transform",`${t.name} → ${(res.entities||[]).length} new`); toast(`+${(res.entities||[]).length} entities`,"ok");
  }catch(e){ setSync("err","failed"); toast(e.message,"err"); }
}
// Build a cytoscape element for one graph node (shared by render + append).
function graphNodeEl(n){ const band=n.band||bandOf(n.risk); const hot=band==="critical"||band==="high";
  return { data:{ id:n.id, label:n.label, icon:nodeIcon(n.kind), kc:kColor(n.kind), hc:bandColor(band), size:24+(n.risk||0)*26, bw:hot?2.5:1.5, halo:hot?1:undefined }, classes:n.hypothesis?"hyp":"" }; }
// Incrementally add nodes/edges near an anchor WITHOUT re-laying-out the whole
// graph — new results appear next to the seed and settle with a small local layout.
function appendToCy(newNodes, newEdges, anchorId){ if(!cy){ renderGraph(); return; }
  const anchor=cy.$id(anchorId); const ap=(anchor&&anchor.length)?anchor.position():{x:0,y:0};
  let added=cy.collection(); let ei=cy.edges().length+1;
  newNodes.forEach((n,i)=>{ if(cy.$id(n.id).length) return; const ne=cy.add(graphNodeEl(n)); const ang=(i/Math.max(1,newNodes.length))*Math.PI*2;
    ne.position({x:ap.x+Math.cos(ang)*100+(Math.random()*24-12), y:ap.y+Math.sin(ang)*100+(Math.random()*24-12)}); added=added.union(ne); });
  newEdges.forEach(e=>{ if(cy.$id(e.source).length&&cy.$id(e.target).length){ const dup=cy.edges().some(x=>x.data("source")===e.source&&x.data("target")===e.target);
    if(!dup) cy.add({ data:{ id:"ex"+(ei++), source:e.source, target:e.target, type:e.type, w:0.6+(e.conf||0.5)*1.8 }, classes:e.hypothesis?"hyp":"" }); } });
  if(added.length){ const region=anchor.union(added).union(added.connectedEdges());
    region.layout({ name:"cose", fit:false, animate:true, animationDuration:400, randomize:false, nodeRepulsion:5000, idealEdgeLength:75,
      boundingBox:{x1:ap.x-240,y1:ap.y-240,x2:ap.x+240,y2:ap.y+240} }).run(); }
  return added;
}
function mergeTransformResult(seed, res){ const tb=activeTab(); if(!tb)return; const byLabel={}; tb.graph.nodes.forEach(n=>byLabel[n.label.toLowerCase()]=n.id);
  const newNodes=[];
  (res.entities||[]).forEach(e=>{ const key=(e.label||"").toLowerCase(); if(!key||byLabel[key])return; const nid="tf-"+Math.abs(hashStr(key+(e.kind||"")+String(tb.graph.nodes.length)));
    byLabel[key]=nid; const node={id:nid,kind:e.kind||"unknown",label:e.label,risk:0.3,band:"low",attributes:e.attributes||{},tags:["transform"],sources:["transform:"+(res._src||"")]}; tb.graph.nodes.push(node); newNodes.push(node); });
  const newEdges=[];
  (res.relationships||[]).forEach(r=>{ const s=byLabel[(r.source||"").toLowerCase()]||seed.id, tg=byLabel[(r.target||"").toLowerCase()]; if(s&&tg){ const edge={source:s,target:tg,type:r.type||"related",conf:r.confidence||0.5}; tb.graph.edges.push(edge); newEdges.push(edge); } });
  // any new node with no link gets attached to the seed so nothing floats isolated
  newNodes.forEach(n=>{ if(!newEdges.some(e=>e.source===n.id||e.target===n.id)){ const edge={source:seed.id,target:n.id,type:"from_transform",conf:0.6}; tb.graph.edges.push(edge); newEdges.push(edge); } });
  const freshIds=newNodes.map(n=>n.id);
  if((tb.clusterMode||"none")==="none" && cy && cy.$id(seed.id).length){
    appendToCy(newNodes,newEdges,seed.id); flashFresh(freshIds);
    setTimeout(()=>{ if(cy) cy.animate({fit:{eles:cy.$id(seed.id).closedNeighborhood(),padding:90},duration:420}); },430);
  } else { renderGraph(); flashFresh(freshIds); }
  renderGraphFilters(); selectNode(seed.id);
}
function flashFresh(ids){ if(!cy||!ids||!ids.length)return; setTimeout(()=>{ ids.forEach(id=>{ const e=cy.$id(id); if(e&&e.length){ e.addClass("fresh"); setTimeout(()=>e.removeClass("fresh"),1800); } }); },100); }

// ---------- intelligence view (decision-grade) ----------
function graphDegrees(g){ const deg={}; g.nodes.forEach(n=>deg[n.id]=0); g.edges.forEach(e=>{ if(deg[e.source]!=null)deg[e.source]++; if(deg[e.target]!=null)deg[e.target]++; }); return deg; }
function computeClusters(g){ const deg=graphDegrees(g); const byId={}; g.nodes.forEach(n=>byId[n.id]=n);
  const adj={}; g.nodes.forEach(n=>adj[n.id]=[]); g.edges.forEach(e=>{ if(adj[e.source]&&adj[e.target]){ adj[e.source].push(e.target); adj[e.target].push(e.source); } });
  const hubs=[...g.nodes].sort((a,b)=>(deg[b.id]||0)-(deg[a.id]||0)).filter(n=>(deg[n.id]||0)>=2).slice(0,8);
  return hubs.map(h=>{ const nb=adj[h.id]||[]; const kinds={}; nb.forEach(id=>{ const k=byId[id]?.kind||"?"; kinds[k]=(kinds[k]||0)+1; });
    const dom=Object.entries(kinds).sort((a,b)=>b[1]-a[1])[0]; const maxRisk=Math.max(h.risk||0,...nb.map(id=>byId[id]?.risk||0));
    return { id:h.id, hub:h, size:nb.length, dominant:dom?dom[0]:"mixed", band:bandOf(maxRisk) }; });
}
// ---- competing hypotheses engine (structural + AI refinement) ----
function structuralHypotheses(g,m){ const H=[]; const deg=m.deg;
  // H1: shared-infrastructure cluster (accounts/persons sharing an IP/domain/device/wallet)
  const hubs=g.nodes.filter(n=>["ip","domain","device","wallet","group"].includes(n.kind)).map(n=>({n,d:deg[n.id]||0})).filter(x=>x.d>=3).sort((a,b)=>b.d-a.d);
  if(hubs.length){ const h=hubs[0]; const lk=Math.min(0.9,0.4+h.d*0.05);
    H.push({title:`Coordinated activity via shared ${h.n.kind} "${h.n.label}"`, likelihood:lk, confidence:h.d>=6?"medium":"low",
      evidence:[`${h.d} entities connect to ${h.n.label}`,"Shared infrastructure is a correlation signal"], missing:["Timestamps to confirm co-occurrence","Ownership/attribution of the hub"],
      next:"Isolate this cluster and expand its members", act:()=>focusEntity(h.n.id,true)}); }
  // H2: duplicate/identity collision
  if(m.duplicates){ H.push({title:`Duplicate or conflated identities (${m.duplicates} candidates)`, likelihood:Math.min(0.85,0.3+m.duplicates*0.03), confidence:"medium",
    evidence:[`${m.dupGroups.length} label collisions across ${m.duplicates} entities`],missing:["Unique identifiers (doc, phone, hash) to merge/split"],
    next:"Review duplicates in Entity Registry", act:()=>{showView("entities");entityFilter="dupes";renderEntities();}}); }
  // H3: risk concentration in few hubs
  const topDeg=g.nodes.map(n=>deg[n.id]||0).sort((a,b)=>b-a); const totDeg=topDeg.reduce((s,x)=>s+x,0)||1; const top5=topDeg.slice(0,5).reduce((s,x)=>s+x,0);
  if(g.nodes.length>20 && top5/totDeg>0.35){ H.push({title:"Risk/connectivity concentrated in a few hubs", likelihood:0.6, confidence:"medium",
    evidence:[`Top 5 nodes hold ${Math.round(top5/totDeg*100)}% of connections`],missing:["Whether hubs are legitimate aggregators or true coordination"],
    next:"Inspect top hubs before drawing conclusions", act:()=>{showView("graph");$("#graphCluster")&&($("#graphCluster").value="kind");setClusterMode("kind");}}); }
  // H4: insufficient data (always a competing hypothesis when quality/coverage low)
  if(m.avgQual<0.55||m.coverage<0.6||m.sourceDiversity<2){ H.push({title:"Insufficient/low-quality data — patterns may be artifacts", likelihood:0.5+ (0.55-Math.min(0.55,m.avgQual)), confidence:"low",
    evidence:[`avg quality ${pct(m.avgQual)}, coverage ${pct(m.coverage)}, ${m.sourceDiversity} source(s)`],missing:["Additional sources","Metadata/timestamps for isolated entities"],
    next:"Enrich data before acting on inferences", act:()=>{showView("entities");entityFilter="missing";renderEntities();}}); }
  return H;
}
function decisionMatrix(m,g,H){ const opts=[]; const total=m.total||1;
  const norm=(v,max)=>Math.max(0,Math.min(1,v/max));
  if(m.highRisk) opts.push({action:`Escalate ${m.highRisk} high-risk entities for review`, impact:0.9, conf:m.avgConf, riskWrong:0.35, effort:0.3, route:"Graph", go:()=>isolateCritical()});
  if(m.duplicates) opts.push({action:`Resolve ${m.duplicates} likely duplicates`, impact:0.55, conf:0.7, riskWrong:0.2, effort:0.45, route:"Entities", go:()=>{showView("entities");entityFilter="dupes";renderEntities();}});
  if(m.missingSource+m.missingMeta) opts.push({action:`Enrich ${m.missingSource+m.missingMeta} entities missing evidence`, impact:0.5, conf:0.8, riskWrong:0.1, effort:0.6, route:"Entities", go:()=>{showView("entities");entityFilter="missing";renderEntities();}});
  const lead=H[0]; if(lead&&lead.act) opts.push({action:`Act on lead hypothesis: ${lead.title}`, impact:0.8, conf:lead.likelihood, riskWrong:1-lead.likelihood, effort:0.4, route:"Graph", go:lead.act});
  opts.push({action:"Gather more data before deciding", impact:0.3, conf:0.9, riskWrong:0.05, effort:0.5, route:"Sources", go:()=>{showView("settings");openSettingsTab("datasources");}});
  // weighted score: reward impact*confidence, penalize risk-if-wrong and effort
  opts.forEach(o=>{ o.score=(0.45*o.impact + 0.30*o.conf - 0.20*o.riskWrong - 0.05*o.effort); });
  opts.sort((a,b)=>b.score-a.score); return opts;
}
function renderHypotheses(t,g,m){ const w=$("#intelHypotheses"); if(!w)return;
  let H=structuralHypotheses(g,m);
  // merge AI-provided hypotheses if present
  const ai=(t.intel&&(t.intel.hypotheses))||[];
  ai.forEach(h=>{ if(typeof h==="string"){ H.push({title:h,likelihood:0.5,confidence:t.intel.confidence||"low",evidence:[],missing:[],next:""}); }
    else if(h&&h.title){ H.push({title:h.title,likelihood:h.likelihood||h.score||0.5,confidence:h.confidence||"low",evidence:h.evidence||[],missing:h.missing_evidence||h.missing||[],next:h.next_action||h.next||""}); } });
  H.sort((a,b)=>b.likelihood-a.likelihood);
  if(!H.length){ w.innerHTML='<div class="empty">No competing hypotheses — add data or generate intelligence.</div>'; return; }
  w.innerHTML="";
  H.slice(0,6).forEach((h,i)=>{ const d=el("div","hyp-card"+(i===0?" lead":"")); const ev=(h.evidence||[]).slice(0,4), ms=(h.missing||[]).slice(0,3);
    d.innerHTML=`<div class="hyp-top"><div class="hyp-title"><span class="rank">H${i+1}</span>${esc(h.title)}${i===0?' <span class="tag ok">lead</span>':''}</div>
      <div class="hyp-scores"><span class="likelihood"><span class="lk-track"><span style="width:${Math.round(h.likelihood*100)}%"></span></span><span class="lk-n">${pct(h.likelihood)}</span></span><span class="chip">conf ${esc(h.confidence)}</span></div></div>
      <div class="hyp-body"><div class="hyp-col"><h5>Supporting evidence</h5><ul>${ev.length?ev.map(x=>`<li>${esc(x)}</li>`).join(""):"<li>—</li>"}</ul></div>
      <div class="hyp-col"><h5>Missing evidence</h5><ul class="missing">${ms.length?ms.map(x=>`<li>${esc(x)}</li>`).join(""):"<li>—</li>"}</ul></div></div>
      ${h.next?`<div class="hyp-foot"><span class="hyp-next">Next: <b>${esc(h.next)}</b></span></div>`:""}`;
    if(h.act){ d.style.cursor="pointer"; d.addEventListener("click",e=>{ if(e.target.tagName!=="A")h.act(); }); }
    w.appendChild(d); });
  t._hyp=H;
}
function renderDecisionMatrix(t,g,m){ const tb=$("#intelDecision tbody"); if(!tb)return; const H=t._hyp||structuralHypotheses(g,m);
  const opts=decisionMatrix(m,g,H); t._decision=opts; tb.innerHTML="";
  const bar=(v,col)=>`<span class="dm-bar"><span style="width:${Math.round(v*100)}%;background:${col}"></span></span>${pct(v)}`;
  opts.forEach((o,i)=>{ const tr=el("tr"); if(i===0)tr.className="best";
    tr.innerHTML=`<td>${esc(o.action)}</td><td>${bar(o.impact,"var(--accent)")}</td><td>${bar(o.conf,"var(--green)")}</td><td>${bar(o.riskWrong,"var(--red)")}</td><td>${bar(o.effort,"var(--amber)")}</td><td class="dm-score">${pct(o.score)}</td><td><span class="chip">${esc(o.route)}</span></td>`;
    tr.style.cursor="pointer"; tr.addEventListener("click",o.go); tb.appendChild(tr); });
  // Next best action
  const nba=$("#intelNba"); if(nba){
    // Prefer the backend's uncertainty-reduction ranking (B2); fall back to the
    // decision-matrix top option.
    const backNba=(activeTab()&&activeTab().graph.meta&&activeTab().graph.meta.nba)||[];
    if(backNba.length){ nba.innerHTML="";
      const top=backNba[0]; nba.innerHTML=`<div class="nba-title">${esc(top.action)}</div>
        <div class="nba-meta"><span class="nba-tag">uncertainty ↓ ${pct(top.uncertainty_reduction)}</span><span class="nba-tag">effort ${pct(top.effort)}</span><span class="nba-tag">priority ${pct(top.priority)}</span><span class="nba-tag">→ ${esc(top.target)}</span></div>
        <div class="nba-residual">Why: ${esc(top.why)}</div>`;
      const goTo=a=>{ if(a.entity_ids&&a.entity_ids.length){ focusEntity(a.entity_ids[0],true); } else if(a.target==="entities"){ showView("entities"); renderEntities&&renderEntities(); } else if(a.target==="sources"){ showView("settings"); openSettingsTab&&openSettingsTab("datasources"); } else { showView("graph"); } };
      const btn=el("button","btn primary"); btn.style.marginTop="10px"; btn.textContent="Take this action"; btn.addEventListener("click",()=>goTo(top)); nba.appendChild(btn);
      if(backNba.length>1){ const more=el("div"); more.style.marginTop="12px";
        backNba.slice(1,5).forEach((a,i)=>{ const r=el("div","li"); r.style.cursor="pointer"; r.innerHTML=`<span class="label">${i+2}. ${esc(a.action)}</span><span class="chip">↓${pct(a.uncertainty_reduction)}</span>`; r.addEventListener("click",()=>goTo(a)); more.appendChild(r); });
        nba.appendChild(more); }
    }
    else if(!opts.length){ nba.innerHTML='<div class="empty">Run an analysis to rank next actions.</div>'; }
    else { const b=opts[0]; const residual=(1-b.conf); nba.innerHTML=`<div class="nba-title">${esc(b.action)}</div>
      <div class="nba-meta"><span class="nba-tag">impact ${pct(b.impact)}</span><span class="nba-tag">confidence ${pct(b.conf)}</span><span class="nba-tag">risk-if-wrong ${pct(b.riskWrong)}</span><span class="nba-tag">route ${esc(b.route)}</span></div>
      <div class="nba-residual">Residual uncertainty ~${pct(residual)} — this is decision support, not certainty. ${m.readiness==="ready"?"Data supports acting now.":"Consider resolving gaps first (see Data Quality)."}</div>
      <button class="btn primary" id="nbaGo" style="margin-top:10px">Take this action</button>`;
      $("#nbaGo")&&$("#nbaGo").addEventListener("click",b.go); } }
}

// Pre-defined analysis flows (one-click prompts that steer the assessment).
const FLOW_COMMON=[
  ["◆","Executive summary","Give a 3-sentence executive summary for a non-technical decision-maker: what's happening, why it matters, and the single most important next step."],
  ["⚠","Biggest risks","What are the top 3 risks in this data, ranked, each with the evidence and how confident we are?"],
  ["◇","What's missing","What data or verification is missing that would most change the conclusion, and why?"],
  ["▶","Action plan","Produce a prioritized action plan: what to do first, second, third, with expected impact and effort."],
];
const FLOW_DOMAIN={
  fraud:[["$","Money-flow","Trace the money flows and shared infrastructure; where is exposure concentrated and who are the likely controllers?"]],
  kyc:[["ID","Identity risk","Assess identity plausibility: which persons/documents look weak or conflated and what verification is needed?"]],
  cybersecurity:[["⌘","Infra & actors","Map infrastructure and actors: shared hosts/IPs/domains, likely operators, and containment leads."]],
  commerce:[["♥","Churn & value","Which customers/segments are highest value or at-risk, and what outreach should happen?"]],
  "child-protection":[["!","Victim priority","Which entities indicate imminent risk or victim-identification leads that need escalation now?"]],
  logistics:[["⛓","Bottlenecks","Where are the operational bottlenecks or single points of failure, and how to make routing resilient?"]],
};
function renderFlows(){ const w=$("#intelFlows"); if(!w)return; const t=activeTab(); w.innerHTML="";
  const flows=[...(FLOW_DOMAIN[t?t.project.domain:""]||[]), ...FLOW_COMMON];
  flows.forEach(([ic,label,prompt])=>{ const b=el("button","flow-btn"); b.innerHTML=`<span class="fb-ic">${ic}</span>${esc(label)}`; b.title=prompt; b.addEventListener("click",()=>generateIntelligence(prompt)); w.appendChild(b); });
}

function renderIntelligence(){ const t=activeTab(); const g=t?t.graph:{nodes:[],edges:[]}; const m=computeMetrics(g);
  renderFlows();
  if(t){ renderHypotheses(t,g,m); renderDecisionMatrix(t,g,m); }
  const top=$("#intelTop"); if(top){ top.innerHTML=""; const nodes=[...g.nodes].sort((a,b)=>b.risk-a.risk).slice(0,12); if(!nodes.length)top.innerHTML='<div class="empty">—</div>';
    nodes.forEach(n=>{ const li=el("div","li"); const l=el("div","l"); const d=el("span","kdot"); d.style.background=kColor(n.kind); l.appendChild(d); l.appendChild(el("span","label",n.label)); li.appendChild(l); li.appendChild(el("span","band "+(n.band||bandOf(n.risk)),(n.band||bandOf(n.risk)))); li.addEventListener("click",()=>focusEntity(n.id,true)); top.appendChild(li); }); }
  const cl=$("#intelClusters"); if(cl){ cl.innerHTML=""; const clusters=computeClusters(g); if(!clusters.length)cl.innerHTML='<div class="empty">No dense clusters yet.</div>';
    clusters.forEach(c=>{ const d=el("div","cluster"); d.innerHTML=`<span><span class="kdot" style="display:inline-block;width:8px;height:8px;border-radius:2px;background:${kColor(c.hub.kind)};margin-right:7px"></span>${esc(c.hub.label)} <span class="conf">· ${c.dominant} hub</span></span><span class="csize">${c.size} <span class="band ${c.band}">${c.band}</span></span>`;
      d.addEventListener("click",()=>focusEntity(c.id,true)); cl.appendChild(d); }); }
  const gaps=$("#intelGaps"); if(gaps){ gaps.innerHTML=""; const deg=graphDegrees(g);
    const sparse=g.nodes.filter(n=>Object.keys(n.attributes||{}).length===0).length;
    const isolated=g.nodes.filter(n=>(deg[n.id]||0)===0).length;
    const hyp=g.nodes.filter(n=>n.hypothesis||(n.tags||[]).includes("hypothesis")).length;
    const sensitiveUnrev=g.nodes.filter(n=>n.sensitive).length;
    const items=[];
    if(!g.nodes.length) items.push("No data ingested — connect a source or run an analysis.");
    if(sparse) items.push(`${sparse} entities have no attributes — enrich via transforms (Settings → Transforms).`);
    if(isolated) items.push(`${isolated} isolated entities — no known relationships; correlation may be incomplete.`);
    if(hyp) items.push(`${hyp} AI-proposed entities are unconfirmed hypotheses — verify before reporting.`);
    if(sensitiveUnrev) items.push(`${sensitiveUnrev} sensitive entities require restricted handling and human review.`);
    if(!items.length) items.push("No major gaps detected in the current graph.");
    items.forEach(x=>{ const g2=el("div","gap"); g2.innerHTML=svg("alerts")+`<span>${esc(x)}</span>`; gaps.appendChild(g2); }); }
  const acts=$("#intelActions"); if(acts){ const risk=t&&t.graph.meta&&t.graph.meta.risk; const items=[...new Set((risk&&risk.assessments||[]).filter(a=>a.recommended_action&&a.recommended_action!=="monitor").map(a=>a.recommended_action))];
    // fold in AI-recommended actions if present
    const intel=t&&t.intel; if(intel&&intel.recommended_actions) intel.recommended_actions.forEach(a=>{ if(!items.includes(a))items.push(a); });
    acts.innerHTML=""; if(!items.length)acts.innerHTML='<div class="empty">Run an analysis or generate intelligence.</div>'; items.slice(0,14).forEach(a=>{ const li=el("div","li"); li.appendChild(el("span","label","▸ "+a)); acts.appendChild(li); }); }
  // Assessment: the deterministic backend product shows immediately; the LLM
  // "generate" enriches the exec summary + competing hypotheses on top.
  const intel=t&&t.intel; const backAsmt=(t&&t.graph.meta&&t.graph.meta.assessment)||[];
  const brief=$("#intelBrief"), jud=$("#intelJudgments"), conf=$("#intelConf");
  if(brief){ if(intel){ brief.innerHTML=`<p>${esc(intel.answer||"")}</p>`; } else if(backAsmt.length){ brief.innerHTML=`<p>${esc(backAsmt[0].statement)}</p>`; } else { brief.innerHTML='<div class="empty">Run an analysis, then Generate for a synthesized assessment.</div>'; } }
  if(conf){ conf.textContent = intel&&intel.confidence? ("confidence: "+intel.confidence) : (backAsmt.length? ("confidence: "+Math.round(backAsmt[0].confidence*100)+"%") : ""); }
  if(jud){ jud.innerHTML="";
    // Prefer LLM key judgments; else show the deterministic assessment statements.
    const llmJs=(intel&&(intel.key_judgments||intel.key_points))||[];
    if(llmJs.length){ llmJs.slice(0,8).forEach((j,i)=>{ const d=el("div","judgment"); const txt=typeof j==="string"?j:(j.text||JSON.stringify(j)); const c=(typeof j==="object"&&j.confidence)?j.confidence:(intel.confidence||""); d.innerHTML=`<b>J${i+1}.</b> ${esc(txt)}${c?`<span class="conf">${esc(c)}</span>`:""}`; jud.appendChild(d); }); }
    else if(backAsmt.length){ backAsmt.slice(0,8).forEach((a,i)=>{ const d=el("div","judgment"); d.innerHTML=`<b>J${i+1}.</b> ${esc(a.statement)} <span class="conf">${Math.round(a.confidence*100)}% · ${esc(a.basis)}</span>`+(a.evidence&&a.evidence.length?`<div class="pts muted" style="margin-top:4px;font-size:11px">evidence: ${esc(a.evidence.join("; "))}</div>`:"");
      if(a.evidence_ids&&a.evidence_ids.length){ d.style.cursor="pointer"; d.addEventListener("click",()=>focusEntity(a.evidence_ids[0],true)); } jud.appendChild(d); }); }
    else jud.innerHTML='<div class="empty">Run an analysis to derive judgments.</div>'; }
}
async function generateIntelligence(customPrompt){ const t=activeTab(); if(!t||!t.graph.nodes.length){toast("Open a project with a graph","err");return;}
  showView("intelligence"); $("#intelBrief").innerHTML='<div class="empty">✦ synthesizing intelligence…</div>'; setSync("busy","intel");
  const base="Act as lead analyst and produce a decision-ready INTELLIGENCE PRODUCT for this graph. Provide: (1) a 2-3 sentence executive assessment in 'answer'; (2) 'key_points' as 4-6 crisp KEY JUDGMENTS, each with a confidence word; (3) 'hypotheses' as an array of COMPETING hypotheses, each {title, likelihood 0..1, confidence, evidence:[...], missing_evidence:[...], next_action}; (4) 'recommended_actions' prioritized; (5) overall 'confidence' (low|medium|high). Never state certainty the data doesn't support; separate confirmed facts from inference.";
  const q = customPrompt&&customPrompt.trim() ? (`Analyst directive: ${customPrompt.trim()}\n\n`+base) : base;
  try{ const res=await runJob("ask",{question:q,domain:t.project.domain,provider:state.provider,graph:{nodes:t.graph.nodes,edges:t.graph.edges},aiInstructions:t.project.ai_instructions});
    t.intel=res; if(customPrompt&&customPrompt.trim()) t.intel._prompt=customPrompt.trim();
    setSync("ok","complete"); pushNotif("ai","Intelligence product generated"); renderIntelligence();
    $("#intelMeta").textContent = `· ${t.graph.nodes.length} entities · ${computeClusters(t.graph).length} clusters`+(customPrompt&&customPrompt.trim()?` · steered`:``);
    applyFocus(res.focus);
  }catch(e){ $("#intelBrief").innerHTML='<div class="empty">error: '+esc(e.message)+'</div>'; setSync("err","failed"); }
}
$("#btnIntelSend")&&$("#btnIntelSend").addEventListener("click",()=>generateIntelligence($("#intelPrompt").value));
$("#intelPrompt")&&$("#intelPrompt").addEventListener("keydown",e=>{ if(e.key==="Enter"){ e.preventDefault(); generateIntelligence($("#intelPrompt").value); } });
$("#btnIntelPdf")&&$("#btnIntelPdf").addEventListener("click",()=>exportReportPdf());
// ---------- report PDF (Typst) + generated-reports list ----------
$("#btnReportRefresh")&&$("#btnReportRefresh").addEventListener("click",()=>{renderReport();renderReports();});
$("#btnReportPdf")&&$("#btnReportPdf").addEventListener("click",()=>exportReportPdf());
async function exportReportPdf(){ const t=activeTab(); if(!t||!t.result){ toast("Run an analysis first","err"); return; }
  if(MODE!=="http"){ toast("PDF export runs in the app or via cortex serve.","err"); return; }
  setSync("busy","report"); toast("Rendering PDF report…");
  try{ const r=await runJob("report_pdf",{project_id:t.project.id});
    const name=(t.project.name.replace(/\s+/g,"_"))+"_intel_"+new Date().toISOString().slice(0,16).replace(/[:T]/g,"")+".pdf";
    t.reports=t.reports||[]; t.reports.unshift({name, path:r.path, at:new Date().toISOString().replace("T"," ").slice(0,16)});
    setSync("ok","complete"); pushNotif("report","PDF report created"); toast("Report created — see Reports","ok");
    showView("reports"); renderReports();
  }catch(e){ setSync("err","failed"); toast("PDF: "+e.message,"err"); }
}
async function downloadReport(path,name){ try{ const resp=await fetch("/api/report/download?path="+encodeURIComponent(path),{headers:{Authorization:"Bearer "+TOKEN}}); if(!resp.ok)throw new Error("not found"); const blob=await resp.blob(); const a=el("a"); a.href=URL.createObjectURL(blob); a.download=name; a.click(); URL.revokeObjectURL(a.href); }catch(e){ toast("Download failed: "+e.message,"err"); } }
function renderReports(){ const w=$("#reportsList"); if(!w)return; const t=activeTab(); const reps=(t&&t.reports)||[];
  w.innerHTML=""; if(!reps.length){ w.innerHTML='<div class="empty">No reports generated yet — click "Generate PDF".</div>'; return; }
  reps.forEach(r=>{ const li=el("div","li"); const l=el("div","l"); l.innerHTML=svg("reports")+`<span class="label">${esc(r.name)}</span>`; li.appendChild(l);
    const b=el("button","btn ghost","⬇ Download"); b.addEventListener("click",()=>downloadReport(r.path,r.name)); const wrap=el("div"); wrap.style.display="flex"; wrap.style.gap="8px"; wrap.style.alignItems="center"; wrap.appendChild(el("span","chip",r.at)); wrap.appendChild(b); li.appendChild(wrap); w.appendChild(li); });
}

// ---------- keyboard ----------
window.addEventListener("keydown",e=>{ const meta=e.metaKey||e.ctrlKey;
  if(meta&&e.key.toLowerCase()==="k"){e.preventDefault();openPalette();}
  else if(meta&&e.key.toLowerCase()==="r"){e.preventDefault();runModal();}
  else if(meta&&e.key.toLowerCase()==="n"){e.preventDefault();newProjectModal();}
  else if(meta&&e.key==="/"){e.preventDefault();openGlobalAsk();}
  else if(e.key==="Escape"){ closePalette(); closeModal(); closeGlobalAsk(); $("#ctxmenu").hidden=true; $("#notifDrawer").hidden=true; closeAllSelects(); if(linkMode){ linkMode=null; const b=$("#linkmodeBanner"); if(b)b.hidden=true; } if(pathSource) clearPath(); } });

// ---------- file helpers ----------
function pickFile(cb){ const inp=$("#filePicker"); inp.removeAttribute("accept"); inp.value=""; inp.onchange=()=>{ const f=inp.files[0]; if(!f)return; const rd=new FileReader(); rd.onload=()=>cb(rd.result); rd.readAsText(f); }; inp.click(); }
let npUploadPath=null;
// Browse a local file, upload its bytes to the server, and return the server-side path.
function browseUpload(cb, accept){ const inp=$("#filePicker"); if(accept)inp.setAttribute("accept",accept); else inp.removeAttribute("accept"); inp.value="";
  inp.onchange=async ()=>{ const f=inp.files[0]; if(!f)return;
    // Only the static artifact preview lacks a backend; a local origin always has one.
    if(!isLocalOrigin() && MODE!=="http"){ cb("/uploads/"+f.name); toast("Preview: file path simulated","ok"); return; }
    setSync("busy","upload"); toast(`Uploading ${f.name} (${(f.size/1048576).toFixed(1)} MB)…`);
    try{ const buf=await f.arrayBuffer();
      const r=await fetch("/api/upload?name="+encodeURIComponent(f.name),{method:"POST",headers:{"Authorization":"Bearer "+TOKEN,"Content-Type":"application/octet-stream"},body:buf});
      const txt=await r.text(); let j; try{ j=JSON.parse(txt);}catch(_){ j={}; }
      if(!r.ok) throw new Error(j.error||("upload failed ("+r.status+")"));
      setSync("ok","uploaded"); cb(j.path); toast("Uploaded "+f.name,"ok"); }
    catch(e){ setSync("err","failed"); toast("Upload failed: "+e.message,"err"); } };
  inp.click(); }

// Server-side file/folder browser. The desktop WebView often won't open a native
// file dialog, so we navigate the local filesystem via /api/fs/list; the embedded
// server reads local paths directly (no upload needed). `opts.accept` is a comma
// list of extensions to highlight; `opts.folders` shows a "Select this folder"
// action. Falls back to the device upload for the static/mock preview.
async function pickServerPath(cb, opts={}){
  if(MODE!=="http"){ return browseUpload(cb, opts.accept); }
  const accept=(opts.accept||"").split(",").map(s=>s.trim().replace(/^\*?\./,"").toLowerCase()).filter(Boolean).filter(s=>!s.includes("/"));
  const matches=name=>{ if(!accept.length) return true; const ext=name.split(".").pop().toLowerCase(); return accept.includes(ext); };
  let cur=null;
  const body=`<div class="fsb"><div class="fsb-path" id="fsbPath">…</div><div class="fsb-list" id="fsbList"></div></div>`;
  const foot=[{label:"Upload from device",cls:"ghost",act:()=>{ closeModal(); browseUpload(cb, opts.accept); }},{label:"Cancel",cls:"ghost",act:closeModal}];
  if(opts.folders) foot.unshift({label:"Select this folder",cls:"primary",act:()=>{ if(cur){ closeModal(); cb(cur); } }});
  openModal(opts.title||"Choose a file", body, foot);
  async function load(path){
    const w=$("#fsbList"); if(!w)return; w.innerHTML='<div class="empty">loading…</div>';
    let data; try{ data=await api("/api/fs/list"+(path?"?path="+encodeURIComponent(path):"")); }catch(e){ w.innerHTML='<div class="empty">'+esc(e.message)+'</div>'; return; }
    cur=data.path; const pe=$("#fsbPath"); if(pe)pe.textContent=data.path;
    w.innerHTML="";
    if(data.parent){ const up=el("div","fsb-row dir"); up.innerHTML=svg("fit")+"<span>.. (up one level)</span>"; up.addEventListener("click",()=>load(data.parent)); w.appendChild(up); }
    data.dirs.forEach(d=>{ const r=el("div","fsb-row dir"); r.innerHTML='<span class="fi">📁</span><span>'+esc(d.name)+'</span>'; r.addEventListener("click",()=>load(d.path)); w.appendChild(r); });
    data.files.filter(f=>matches(f.name)).forEach(f=>{ const r=el("div","fsb-row file"); r.innerHTML='<span class="fi">📄</span><span>'+esc(f.name)+'</span><span class="fsz">'+(f.size>1048576?(f.size/1048576).toFixed(1)+" MB":Math.max(1,Math.round(f.size/1024))+" KB")+'</span>'; r.addEventListener("click",()=>{ closeModal(); cb(f.path); }); w.appendChild(r); });
    if(!w.children.length) w.innerHTML='<div class="empty">empty folder</div>';
  }
  setTimeout(()=>load(opts.start||null),40);
}
function downloadText(name,text){ const b=new Blob([text],{type:"application/json"}); const a=el("a"); a.href=URL.createObjectURL(b); a.download=name; a.click(); URL.revokeObjectURL(a.href); }
// Copy helper with a WebView-safe fallback (clipboard API is often blocked in the
// desktop WebView, so fall back to a hidden textarea + execCommand).
async function copyToClipboard(text){
  try{ if(navigator.clipboard&&navigator.clipboard.writeText){ await navigator.clipboard.writeText(text); return true; } }catch(e){}
  try{ const ta=el("textarea"); ta.value=text; ta.style.cssText="position:fixed;left:-9999px;top:0"; document.body.appendChild(ta); ta.select(); const ok=document.execCommand("copy"); ta.remove(); return ok; }catch(e){ return false; }
}
// Append a small "Copy" affordance to a message bubble.
function addCopyBtn(bubble, text){ const b=el("span","msg-copy","⧉ copy"); b.title="Copy text";
  b.addEventListener("click",async()=>{ const ok=await copyToClipboard(text); toast(ok?"Copied":"Copy failed",ok?"ok":"err"); b.textContent=ok?"✓ copied":"⧉ copy"; setTimeout(()=>b.textContent="⧉ copy",1500); });
  bubble.appendChild(b); }

// ---------- mock backend (artifact preview / static) ----------
function mockApi(path, method, body){
  const P="child-protection";
  if(path==="/api/health") return Promise.resolve({cortex:true,modules:["ingestion","normalization","entity-extraction","graph-correlation","risk-prioritization","investigation","audit","connectors","ai-copilot"],backends:[{name:"claude",ok:true,detail:"preview"},{name:"codex",ok:true,detail:"preview"},{name:"mock",ok:true,detail:"offline"}],plugins:[],has_accounts:true});
  if(path==="/api/auth/status") return Promise.resolve({has_accounts:true});
  if(path.startsWith("/api/auth/")) return Promise.resolve({token:"mock",user:{id:"u",email:"demo@cortex.local",display_name:"Demo Analyst",role:"admin"}});
  if(path==="/api/me") return Promise.resolve({id:"u",email:"demo@cortex.local",display_name:"Demo Analyst",role:"admin"});
  if(path==="/api/domains") return Promise.resolve([["child-protection","Child Protection & Victim Identification"],["cybersecurity","Cybersecurity / Threat Intelligence"],["fraud","Fraud, AML & Financial Crime"],["health","Healthcare & Clinical Safety"],["commerce","Commerce & Retail Decisioning"],["logistics","Logistics & Supply Chain"],["generic","Generic Intelligence"]].map(([slug,title])=>({slug,title,mission:""})));
  if(path==="/api/data_types") return Promise.resolve(["case","report","media","account","person","device","network","url","communication","financial","location","customer","student","employee","product","order","shipment","asset","sensor","log","event","generic"].map(s=>({slug:s})));
  if(path==="/api/agents") return Promise.resolve([]);
  if(path==="/api/doctor") return Promise.resolve([{name:"claude",ok:true,detail:"preview"},{name:"codex",ok:true,detail:"preview"},{name:"mock",ok:true,detail:"offline"}]);
  if(path==="/api/plugins") return Promise.resolve([]);
  if(path==="/api/projects" && method!=="POST") return Promise.resolve([{id:"demo",name:"Demo Investigation",domain:P,updated_at:0,activity_count:3,connector_count:0,has_result:true}]);
  if(path==="/api/projects" && method==="POST") return Promise.resolve({id:"demo",name:body.name||"Demo",domain:body.domain||P,activities:[],connectors:[],last_result:null});
  if(path.startsWith("/api/projects/get")) return Promise.resolve(MOCK_PROJECT);
  if(path==="/api/run"||path==="/api/connectors/run") return Promise.resolve(MOCK_PROJECT.last_result);
  if(path==="/api/ask") return Promise.resolve({answer:"(preview) In live mode this routes your question through Claude/Codex with the current graph as context and returns explainable intelligence — plus proposed entities/links to expand the graph.",key_points:["Accounts darkfox & nightowl share IP 203.0.113.9","Onion host linked to known-hash media"],recommended_actions:["Preserve platform logs","Prioritize victim identification"],entities:[{kind:"account",label:"ghostfox",hypothesis:true}],relationships:[{source:"ghostfox",target:"203.0.113.9",type:"possible_same_ip",confidence:0.5,hypothesis:true}]});
  if(path.startsWith("/api/connectors/test")) return Promise.resolve({status:"preview — connect in live mode"});
  if(path.startsWith("/api/projects/import")) return Promise.resolve(MOCK_PROJECT);
  return Promise.resolve({});
}
const MOCK_NODES=[
  {id:"e1",kind:"case",label:"C-500 distribution ring",risk_score:0.98,risk_band:"critical",tags:["priority"],attributes:{case_type:"distribution network"},sources:["reports.csv#0"]},
  {id:"e2",kind:"account",label:"darkfox",risk_score:0.95,risk_band:"critical",tags:[],attributes:{platform_name:"Telegram"},sources:["reports.csv#0"]},
  {id:"e3",kind:"account",label:"nightowl",risk_score:0.8,risk_band:"high",tags:[],attributes:{platform_name:"Discord"},sources:["reports.csv#1"]},
  {id:"e4",kind:"ip",label:"203.0.113.9",risk_score:0.95,risk_band:"critical",tags:["shared"],attributes:{},sources:["reports.csv#0"]},
  {id:"e5",kind:"media",label:"media:f3db19729d1f",risk_score:0.9,risk_band:"critical",tags:["known-hash"],attributes:{},sources:["reports.csv#0"],sensitive:true},
  {id:"e6",kind:"person",label:"Alex Doe",risk_score:0.85,risk_band:"critical",tags:[],attributes:{},sources:["reports.csv#0"]},
  {id:"e7",kind:"wallet",label:"0x0011…2233",risk_score:0.7,risk_band:"high",tags:[],attributes:{},sources:["reports.csv#0"]},
  {id:"e8",kind:"victim",label:"victim:partial-01",risk_score:0.8,risk_band:"high",tags:["identify"],attributes:{},sources:["reports.csv#1"],sensitive:true},
];
const MOCK_PROJECT={ id:"demo",name:"Demo Investigation",domain:"child-protection",description:"sample",activities:[{id:"a",kind:"run",summary:"Analysis: 8 entities",at:0}],connectors:[],
  last_result:{ entities:{all:MOCK_NODES}, relationships:[
    {source_id:"e1",rel_type:"has_report",target_id:"e2",confidence:0.9},{source_id:"e2",rel_type:"logged_in_from_ip",target_id:"e4",confidence:0.7},
    {source_id:"e3",rel_type:"logged_in_from_ip",target_id:"e4",confidence:0.7},{source_id:"e2",rel_type:"same_ip_as",target_id:"e3",confidence:0.5},
    {source_id:"e6",rel_type:"owns_account",target_id:"e2",confidence:0.8},{source_id:"e2",rel_type:"paid",target_id:"e7",confidence:0.6},{source_id:"e2",rel_type:"contacted",target_id:"e8",confidence:0.6},{source_id:"e5",rel_type:"part_of_case",target_id:"e1",confidence:0.8} ],
    ai_assessments:{case_risk_score:0.98,case_risk_band:"critical",assessments:MOCK_NODES.map(n=>({entity_id:n.id,entity_label:n.label,entity_kind:n.kind,risk_score:n.risk_score,risk_band:n.risk_band,recommended_action:"escalate",requires_human_review:true}))},
    investigation:{summary:"Critical distribution ring; darkfox & nightowl correlate via shared IP.",key_findings:["Shared-IP correlation","Known-hash media"],next_steps:[{action:"Preserve platform logs",requires_authorization:true}]},
    governance:{audit_summary:{summary:"2 sensitive entities touched."},retention:{retention_days:365,disposal_date:"2027-07-05",legal_basis:"internal_authorization"}},
    audit_events:[{timestamp:"2026-07-05T21:56:51",action_performed:"ingest_records",stage:"ingestion",entity_scope:"3 records"},{timestamp:"2026-07-05T21:56:54",action_performed:"run_ai_assessment",stage:"risk",entity_scope:"critical"}] } };

// ---------- go ----------
boot();
