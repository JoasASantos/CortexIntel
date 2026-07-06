/* ===== CortexIntel GUI (v2) ===== */
"use strict";

// ---------- transport ----------
const TAURI = window.__TAURI__ || null;
let MODE = "mock"; // "http" | "mock"
let TOKEN = localStorage.getItem("cortex_token") || null;

async function detectTransport() {
  if (typeof location !== "undefined" && /^https?:$/.test(location.protocol)) {
    // Retry: the native app loads the window while the embedded server is still binding.
    for (let i = 0; i < 6; i++) {
      try { const r = await fetch("/api/ping", { cache: "no-store" }); if (r.ok && (await r.json()).cortex) { MODE = "http"; return; } } catch (e) {}
      await new Promise(r => setTimeout(r, 200));
    }
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

// ---------- state ----------
const state = {
  user: null, domains: [], dataTypes: [], provider: "auto",
  tabs: [], active: -1, notifications: [],
};
const KIND_COLOR = {
  case:"#a78bfa", report:"#c084fc", person:"#38bdf8", victim:"#f472b6", suspect:"#ef4444",
  account:"#22d3ee", device:"#2dd4bf", ip:"#f59e0b", url:"#818cf8", domain:"#60a5fa",
  media:"#fb7185", evidence:"#fda4af", communication:"#4ade80", group:"#facc15", payment:"#34d399",
  wallet:"#10b981", location:"#fbbf24", organization:"#93c5fd", malware:"#dc2626",
  vulnerability:"#f97316", incident:"#e879f9", service:"#5eead4", repository:"#a3e635", unknown:"#94a3b8"
};
const kColor = k => KIND_COLOR[k] || KIND_COLOR.unknown;
const KIND_SHAPE = { person:"ellipse", victim:"ellipse", suspect:"ellipse", account:"round-rectangle", device:"round-rectangle",
  ip:"diamond", url:"hexagon", domain:"hexagon", media:"round-tag", evidence:"round-tag", wallet:"pentagon", payment:"pentagon",
  group:"octagon", case:"round-rectangle", report:"round-rectangle", malware:"star", incident:"star", vulnerability:"star",
  location:"triangle", organization:"barrel", service:"round-rectangle", repository:"round-rectangle" };
const kShape = k => KIND_SHAPE[k] || "ellipse";
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
  set('#btnRun','play','Run'); set('#btnAsk','spark','Ask AI');
  const bn=$('#btnNotifications'); if(bn){ bn.innerHTML=svg('bell')+'<span class="badge" id="notifBadge" hidden>0</span>'; }
  const sl=$('.splash-logo'); if(sl)sl.innerHTML=`<svg class="ic" style="width:52px;height:52px" viewBox="0 0 24 24">${ICONS.logo}</svg>`;
  const ge=$('.ge-icon'); if(ge)ge.innerHTML=`<svg class="ic" style="width:44px;height:44px" viewBox="0 0 24 24">${ICONS.graph}</svg>`;
}

// ---------- boot ----------
async function boot() {
  applyIcons();
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
  // Onboarding: ask country (BR/US) once.
  try { const cfg=await api("/api/config"); state.country=cfg.country; if(!cfg.onboarded) onboardCountry(); } catch(e){}
  await loadProjects();
  if (!state.tabs.length) {
    // open most recent project or prompt to create
    const list = await api("/api/projects").catch(()=>[]);
    if (list.length) openProject(list[0].id);
    else showView("dashboard");
  }
}

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
function onboardCountry(){
  openModal("Welcome — set your region", `
    <p class="muted">CortexIntel tailors identity/KYC checks and disclaimers to your country. Supported now: Brazil & United States.</p>
    <div class="country-grid">
      <div class="country-opt sel" data-c="BR"><div class="flag">🇧🇷</div><div class="cn">Brazil</div><div class="muted" style="margin:0">CPF · LGPD</div></div>
      <div class="country-opt" data-c="US"><div class="flag">🇺🇸</div><div class="cn">United States</div><div class="muted" style="margin:0">SSN · privacy</div></div>
    </div>
    <div class="disclaimer">Person/identity data is regulated. Processing requires a lawful basis under LGPD (BR) / GDPR & state law (US). Validation is decision-support, never a definitive identity ruling.</div>
  `,[{label:"Continue",cls:"primary",act:async()=>{ try{ await api("/api/config",{method:"POST",body:{country:onboardSel,onboarded:true}}); state.country=onboardSel; }catch(e){} closeModal(); toast("Region set: "+onboardSel,"ok"); }}]);
  onboardSel="BR";
  setTimeout(()=>{ $$(".country-opt").forEach(o=>o.addEventListener("click",()=>{ $$(".country-opt").forEach(x=>x.classList.remove("sel")); o.classList.add("sel"); onboardSel=o.dataset.c; })); },40);
}
function newProjectModal() {
  const domainOpts = state.domains.map(d=>`<option value="${d.slug}">${esc(d.title)}</option>`).join("");
  openModal("New project", `
    <div class="field">Project name<input id="npName" placeholder="e.g. CourseStack Students" /></div>
    <div class="field">Business vertical<select id="npDomain" class="select">${domainOpts}</select></div>
    <div class="field">Description<textarea id="npDesc" rows="2" placeholder="what this project investigates"></textarea></div>
    <div class="field">Instruction for AI (optional)<textarea id="npAI" rows="2" placeholder="focus/context for the AI, e.g. 'prioritize shared-infrastructure fraud rings; ignore internal test accounts'"></textarea></div>
    <div class="field">Import a file now (optional)<div style="display:flex;gap:8px"><input id="npFile" placeholder="no file selected" readonly style="flex:1" /><button class="btn ghost" id="npBrowse">Browse…</button></div></div>
    <div class="modal-note" id="npNote"></div>
  `, [
    {label:"Cancel", cls:"ghost", act:closeModal},
    {label:"Create", cls:"primary", act: async ()=>{
      const name=$("#npName").value.trim(); if(!name){ toast("Name required","err"); return; }
      try {
        const p = await api("/api/projects",{method:"POST",body:{name,domain:$("#npDomain").value,description:$("#npDesc").value,ai_instructions:$("#npAI").value}});
        closeModal(); await loadProjects(); await openProject(p.id); pushNotif("project",`Project "${p.name}" created`);
        if(npUploadPath){ const tb=activeTab(); if(tb){ setSync("busy","running"); try{ const result=await api("/api/run",{method:"POST",body:{inputs:[npUploadPath],domain:p.domain,provider:state.provider,maxRecords:4000,projectId:p.id}}); tb.result=result; tb.graph=consolidatedToGraph(result); tb.project=await api(`/api/projects/get?id=${encodeURIComponent(p.id)}`).catch(()=>tb.project); setSync("ok","complete"); renderAll(); showView("graph"); setTimeout(()=>{initCy();if(cy)cy.fit(cy.elements(),50);},700);}catch(e){setSync("err","failed");toast(e.message,"err");} } }
      } catch(e){ toast(e.message,"err"); }
    }}
  ]);
  npUploadPath=null;
  setTimeout(()=>{ const b=$("#npBrowse"); if(b) b.addEventListener("click",()=>browseUpload(path=>{ npUploadPath=path; $("#npFile").value=path.split("/").pop(); }, ".csv,.tsv,.json,.jsonl,.ndjson")); },40);
  setTimeout(()=>$("#npName")&&$("#npName").focus(),50);
}

// ---------- graph data ----------
function consolidatedToGraph(c) {
  let nodes=[];
  Object.values(c.entities||{}).forEach(a=>{ if(Array.isArray(a)) nodes=nodes.concat(a); });
  nodes = nodes.map(n=>({ id:n.id, kind:n.kind, label:n.label, risk:n.risk_score||0, band:n.risk_band||bandOf(n.risk_score||0),
    attributes:n.attributes||{}, tags:n.tags||[], sources:n.sources||[], sensitive:!!n.sensitive }));
  const edges=(c.relationships||[]).map(r=>({source:r.source_id,target:r.target_id,type:r.rel_type,conf:r.confidence||0.5}));
  return {nodes, edges, meta:{risk:c.ai_assessments,investigation:c.investigation,governance:c.governance,audit:c.audit_events||[]}};
}

// ---------- cytoscape ----------
let cy = null;
function initCy() {
  if (cy) return cy;
  try { if (window.cytoscapeFcose) cytoscape.use(window.cytoscapeFcose); } catch(e){}
  cy = cytoscape({
    container: $("#cy"),
    wheelSensitivity: 0.25,
    style: [
      { selector:"node", style:{
        "background-color":"data(color)", "width":"data(size)", "height":"data(size)", "shape":"data(shape)",
        "label":"data(label)", "font-size":"9px", "font-family":"var(--mono)", "color":"#aeb9c6", "text-wrap":"ellipsis",
        "text-max-width":"90px", "text-valign":"bottom", "text-margin-y":4, "min-zoomed-font-size":7,
        "border-width":"data(bw)", "border-color":"data(bc)", "border-opacity":0.9,
        "transition-property":"opacity background-color border-width", "transition-duration":"180ms" }},
      { selector:"node[halo]", style:{ "underlay-color":"data(bc)", "underlay-padding":6, "underlay-opacity":0.35 }},
      { selector:"node:selected", style:{ "border-width":3, "border-color":"#ffffff", "underlay-color":"#33c2dd", "underlay-padding":8, "underlay-opacity":0.4 }},
      { selector:"edge", style:{
        "width":"data(w)", "line-color":"rgba(120,140,165,0.22)", "target-arrow-color":"rgba(120,140,165,0.3)",
        "target-arrow-shape":"triangle", "arrow-scale":0.7, "curve-style":"bezier",
        "label":"data(type)", "font-size":"7px", "font-family":"var(--mono)", "color":"rgba(150,170,190,0.5)", "text-rotation":"autorotate", "min-zoomed-font-size":10,
        "transition-property":"opacity line-color", "transition-duration":"180ms" }},
      { selector:"edge:selected", style:{ "line-color":"#33c2dd", "width":2 }},
      { selector:".hyp", style:{ "line-style":"dashed", "line-color":"#8b7fe8", "border-color":"#8b7fe8", "border-style":"dashed" }},
      { selector:".faded", style:{ "opacity":0.1 }},
      { selector:".fresh", style:{ "underlay-color":"#3fb457", "underlay-padding":10, "underlay-opacity":0.5 }},
    ],
  });
  cy.on("tap","node", ev=>selectNode(ev.target.id()));
  cy.on("tap", ev=>{ if(ev.target===cy){ cy.elements().removeClass("faded"); cy.$(":selected").unselect(); $("#context").hidden=true; } });
  cy.on("cxttap","node", ev=>{ const e=ev.originalEvent; if(linkMode) finishLink(ev.target.id()); else openCtxMenu(e.clientX,e.clientY,ev.target.id()); });
  cy.on("tap","node", ev=>{ if(linkMode) finishLink(ev.target.id()); });
  return cy;
}

function renderGraph() {
  const t = activeTab();
  const container = $("#cy");
  $("#graphEmpty").hidden = !!(t && t.graph.nodes.length);
  if (!t || !t.graph.nodes.length) { if (cy) cy.elements().remove(); $("#graphStats").textContent="0 nodes · 0 edges"; return; }
  initCy();
  const g = t.graph;
  const nodeById = {}; g.nodes.forEach(n=>nodeById[n.id]=n);
  const els = [];
  g.nodes.forEach(n=>{
    const band = n.band||bandOf(n.risk);
    const hot = band==="critical"||band==="high";
    els.push({ data:{ id:n.id, label:n.label, color:kColor(n.kind), size:14+(n.risk||0)*24, shape:kShape(n.kind),
      bw:hot?2.5:1, bc:hot?bandColor(band):"rgba(0,0,0,0.25)", halo:hot?1:undefined }, classes: n.hypothesis?"hyp":"" });
  });
  g.edges.forEach((e,i)=>{ if(nodeById[e.source]&&nodeById[e.target]) els.push({ data:{ id:"e"+i, source:e.source, target:e.target, type:e.type, w:0.6+(e.conf||0.5)*1.8 }, classes:e.hypothesis?"hyp":"" }); });
  cy.elements().remove(); cy.add(els);
  runLayout();
  $("#graphStats").textContent = `${g.nodes.length} nodes · ${g.edges.length} edges`;
  renderLegend(); renderGraphFilters();
}
function runLayout() {
  if (!cy) return;
  const name = $("#graphLayout").value || "fcose";
  const opts = name==="fcose"
    ? { name:"fcose", animate:true, animationDuration:600, randomize:true, nodeRepulsion:8000, idealEdgeLength:70, padding:40 }
    : { name, animate:true, padding:40 };
  try { cy.layout(opts).run(); } catch(e){ cy.layout({name:"cose",animate:true}).run(); }
}
function clearGraph(){ if(cy) cy.elements().remove(); }

function renderLegend() {
  const t = activeTab(); const nodes=(t?.graph.nodes||[]); const counts={};
  nodes.forEach(n=>counts[n.kind]=(counts[n.kind]||0)+1);
  const lg=$("#legend"); lg.innerHTML="";
  Object.entries(counts).sort((a,b)=>b[1]-a[1]).forEach(([k,c])=>{ const x=el("div","lg"); const d=el("span","kdot"); d.style.background=kColor(k); x.appendChild(d); const s=el("span"); s.innerHTML=`${k} <b>${c}</b>`; x.appendChild(s); lg.appendChild(x); });
}

// ---------- entity selection ----------
function nodeData(id){ const t=activeTab(); return t? t.graph.nodes.find(n=>n.id===id):null; }
function selectNode(id) {
  const n = nodeData(id); if(!n) return;
  if (cy) { cy.$(":selected").unselect(); const nel=cy.$id(id); if(nel&&nel.length){ nel.select(); cy.elements().addClass("faded"); nel.closedNeighborhood().removeClass("faded"); } }
  const c=$("#context"); c.hidden=false;
  $("#ctxKind").textContent = n.kind + (n.sensitive?" · sensitive":"");
  $("#ctxName").textContent = n.label;
  const band=n.band||bandOf(n.risk);
  $("#ctxRisk").innerHTML = `<span class="band ${band}">${band} · ${(n.risk||0).toFixed(2)}</span><div class="risk-bar"><span style="width:${Math.round((n.risk||0)*100)}%;background:${bandColor(band)}"></span></div>`;
  const tags=$("#ctxTags"); tags.innerHTML = n.tags.length?"":'<span class="chip">none</span>'; n.tags.forEach(x=>tags.appendChild(el("span","chip",x)));
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
  add("entities","Open dossier",()=>selectNode(id));
  add("spark","Expand via AI",()=>askAbout(`Expand the investigation around "${n.label}" (${n.kind}). Propose linked entities and leads.`));
  add("fit","Isolate neighborhood",()=>isolate(id));
  add("graph","Focus neighbors",()=>{if(cy)cy.animate({fit:{eles:cy.$id(id).closedNeighborhood(),padding:80},duration:350});});
  add("link","Connect to another node…",()=>startLink(id),true);
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
  if(name==="graph"){ requestAnimationFrame(()=>{ initCy(); if(cy) cy.resize(); }); } }
$$(".nav li").forEach(li=>li.addEventListener("click",()=>showView(li.dataset.view)));

function renderAll(){ renderGraph(); renderDashboard(); renderEntities(); renderReport(); renderTimeline(); renderAlerts(); renderSavedConnectors(); renderIntelligence(); }

function renderDashboard(){
  const t=activeTab();
  $("#dashTitle").textContent = t? t.project.name : "Dashboard";
  $("#dashSub").textContent = t? `${t.project.domain} · ${t.project.description||"no description"}` : "Open or create a project to begin.";
  const nodes=t?t.graph.nodes:[]; const edges=t?t.graph.edges:[];
  $("#kpiEntities").textContent=nodes.length; $("#kpiRels").textContent=edges.length;
  $("#kpiCrit").textContent=nodes.filter(n=>(n.band||bandOf(n.risk))==="critical").length;
  $("#kpiActs").textContent=t?t.project.activities.length:0;
  const cl=$("#criticalList"); cl.innerHTML=""; const top=[...nodes].sort((a,b)=>b.risk-a.risk).slice(0,10);
  if(!top.length) cl.innerHTML='<div class="empty">Run an analysis to populate.</div>';
  top.forEach(n=>{ const li=el("div","li"); const l=el("div","l"); const d=el("span","kdot"); d.style.background=kColor(n.kind); l.appendChild(d); l.appendChild(el("span","label",n.label)); li.appendChild(l); li.appendChild(el("span","band "+(n.band||bandOf(n.risk)),(n.band||bandOf(n.risk)))); li.addEventListener("click",()=>focusEntity(n.id,true)); cl.appendChild(li); });
  const al=$("#activityList"); al.innerHTML="";
  const acts=t? [...t.project.activities].reverse().slice(0,10):[];
  if(!acts.length) al.innerHTML='<div class="empty">—</div>';
  acts.forEach(a=>{ const li=el("div","li"); li.appendChild(el("span","label",a.summary)); li.appendChild(el("span","chip",a.kind)); al.appendChild(li); });
  loadProjects();
}
function renderEntities(){ const t=activeTab(); const tb=$("#entitiesTable tbody"); tb.innerHTML="";
  const nodes=t?[...t.graph.nodes].sort((a,b)=>b.risk-a.risk):[];
  nodes.forEach(n=>{ const tr=el("tr"); const b=n.band||bandOf(n.risk);
    const r=el("td"); r.appendChild(el("span","band "+b,(n.risk||0).toFixed(2))); tr.appendChild(r);
    tr.appendChild(el("td",null,n.kind)); tr.appendChild(el("td",null,n.label)); tr.appendChild(el("td",null,(n.tags||[]).join(", ")||"—"));
    tr.addEventListener("click",()=>selectNode(n.id)); tb.appendChild(tr); }); }
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
    <div class="field">LLM provider<select id="rProvider" class="select">
      <option value="auto">Auto — smart routing (Opus/Sonnet ⇄ Codex)</option><option value="claude">Claude (Opus/Sonnet)</option><option value="codex">Codex (gpt-5.5)</option><option value="mock">Offline mock</option></select></div>
    <div class="field">Input source(s)<div style="display:flex;gap:8px"><input id="rInputs" placeholder="/path/file.csv  (or Browse)" style="flex:1" /><button class="btn ghost" id="rBrowse">Browse…</button></div></div>
    <div class="field">Max records (graph cap)<input id="rMax" type="number" value="4000" /></div>
    ${MODE==="mock"?'<div class="modal-note">Preview mode: loads the embedded sample.</div>':''}
  `,[
    {label:"Cancel",cls:"ghost",act:closeModal},
    {label:"▶ Run",cls:"primary",act:doRun}
  ]);
  setTimeout(()=>{ const b=$("#rBrowse"); if(b) b.addEventListener("click",()=>browseUpload(path=>{ const cur=$("#rInputs").value.trim(); $("#rInputs").value=(cur?cur+" ":"")+path; }, ".csv,.tsv,.json,.jsonl,.ndjson")); },40);
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
    const result = await api("/api/run",{method:"POST",body:params});
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
$("#btnAsk").addEventListener("click",openAsk);
$("#btnAsk2").addEventListener("click",openAsk);
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
    const res = await api("/api/ask",{method:"POST",body:{question:q, domain:t?t.project.domain:"generic", provider:state.provider, graph, aiInstructions:t?t.project.ai_instructions:""}});
    thinking.remove();
    const a=el("div","ask-msg a");
    let h=`<div>${esc(res.answer||"(no answer)")}</div>`;
    if(res.key_points&&res.key_points.length){ h+='<ul class="pts">'+res.key_points.map(p=>`<li>${esc(p)}</li>`).join("")+'</ul>'; }
    if(res.recommended_actions&&res.recommended_actions.length){ h+='<ul class="pts">'+res.recommended_actions.map(p=>`<li>▸ ${esc(p)}</li>`).join("")+'</ul>'; }
    const adds=(res.entities&&res.entities.length)||(res.relationships&&res.relationships.length);
    if(adds){ const n=(res.entities||[]).length,r=(res.relationships||[]).length; h+=`<div class="adds" id="addProp">＋ Add ${n} entities / ${r} relations to graph</div>`; }
    a.innerHTML=h; log.appendChild(a); log.scrollTop=log.scrollHeight;
    if(adds){ $("#addProp").addEventListener("click",()=>mergeProposals(res)); }
    applyFocus(res.focus);
    pushNotif("ai","AI copilot answered a query");
  } catch(e){ thinking.remove(); const a=el("div","ask-msg a"); a.textContent="✦ error: "+e.message; log.appendChild(a); }
}
function mergeProposals(res){
  const t=activeTab(); if(!t) return;
  const byLabel={}; t.graph.nodes.forEach(n=>byLabel[n.label.toLowerCase()]=n.id);
  (res.entities||[]).forEach(e=>{ const key=(e.label||"").toLowerCase(); if(!key||byLabel[key])return;
    const id="ai-"+Math.abs(hashStr(key)); byLabel[key]=id;
    t.graph.nodes.push({id,kind:(e.kind||"unknown"),label:e.label,risk:0.4,band:"medium",attributes:e.attributes||{},tags:["hypothesis"],sources:["ai-copilot"],hypothesis:!!e.hypothesis}); });
  (res.relationships||[]).forEach(r=>{ const s=byLabel[(r.source||"").toLowerCase()],tg=byLabel[(r.target||"").toLowerCase()]; if(s&&tg) t.graph.edges.push({source:s,target:tg,type:r.type||"related",conf:r.confidence||0.5,hypothesis:!!r.hypothesis}); });
  renderGraph(); toast("Added AI proposals to graph","ok");
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
  showView("graph"); requestAnimationFrame(()=>{ initCy(); if(!cy)return; cy.resize();
    if(focus.action==="isolate"){ cy.nodes().forEach(nd=>nd.style("display",keep.has(nd.id())?"element":"none")); cy.fit(cy.nodes(":visible"),60); }
    else { cy.elements().addClass("faded"); keep.forEach(id=>{ const e=cy.$id(id); if(e){ e.removeClass("faded"); e.connectedEdges().removeClass("faded"); } }); cy.fit(cy.nodes(":visible").length?cy.nodes().filter(n=>keep.has(n.id())):cy.elements(),60); }
  });
  toast(`AI focus applied — ${keep.size} entities`,"ok");
}

// ---------- connectors ----------
const CONNECTORS=[
  {kind:"csv",name:"CSV / TSV file",desc:"Import a delimited file and auto-process it."},
  {kind:"json",name:"JSON / JSONL",desc:"Import JSON records and expand classification."},
  {kind:"postgres",name:"PostgreSQL",desc:"Connect by host/IP, user & password; run a query."},
  {kind:"mysql",name:"MySQL / MariaDB",desc:"Connect by host/IP, user & password; run a query."},
  {kind:"bigquery",name:"Google BigQuery",desc:"Query BigQuery via the bq CLI."},
  {kind:"datalake",name:"Data lake (S3 / GCS / local)",desc:"Pull CSV/JSON from a bucket or path."},
];
function renderConnectorCards(){ const w=$("#connectorCards"); if(!w)return; w.innerHTML="";
  CONNECTORS.forEach(c=>{ const card=el("div","card conn"); card.innerHTML=`<div class="ct">⇄ ${esc(c.name)}</div><div class="cd">${esc(c.desc)}</div>`; card.addEventListener("click",()=>connectorModal(c)); w.appendChild(card); }); }
function connectorModal(c){
  let fields="";
  if(c.kind==="csv"||c.kind==="json"){ fields=`<div class="field">File path<input id="cPath" placeholder="/opt/CourseStackIntelligence/Students.csv" /></div>`; }
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
    try{ const result=await api("/api/run",{method:"POST",body:params}); t.result=result; t.graph=consolidatedToGraph(result); t.project=await api(`/api/projects/get?id=${encodeURIComponent(t.project.id)}`).catch(()=>t.project); setSync("ok","complete"); renderAll(); showView("graph"); setTimeout(()=>{initCy();if(cy)cy.fit(cy.elements(),50);},700); pushNotif("import",`Imported ${cfg.path}`); toast("Imported & processed","ok"); }
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
$("#btnReset").addEventListener("click",()=>{ if(cy){ cy.nodes().style("display","element"); cy.$(":selected").unselect(); cy.fit(cy.elements(),50); } $("#graphFilter").value=""; $("#context").hidden=true; toast("View reset"); });
$("#graphLayout").addEventListener("change",runLayout);
$("#graphFilter").addEventListener("input",e=>{ const q=e.target.value.trim().toLowerCase(); if(!cy)return;
  if(!q){ cy.nodes().style("display","element"); } else { cy.nodes().forEach(n=>{ const nd=nodeData(n.id()); const show=nd&&(nd.label+" "+nd.kind).toLowerCase().includes(q); n.style("display",show?"element":"none"); }); }
});
$("#globalSearch").addEventListener("keydown",e=>{ if(e.key==="Enter"){ const q=e.target.value.trim().toLowerCase(); const t=activeTab(); const hit=t&&t.graph.nodes.find(n=>n.label.toLowerCase().includes(q)); if(hit) selectNode(hit.id); else toast("No match"); } });

// ---------- command palette ----------
const COMMANDS=[
  ["New project","⌘N",newProjectModal],["Run analysis","⌘R",runModal],["Ask AI copilot","⌘/",openAsk],
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
$$('.nav li').forEach(li=>li.addEventListener("click",()=>{ if(li.dataset.view==="settings")openSettingsTab(currentSettingsTab); if(li.dataset.view==="intelligence")renderIntelligence(); }));
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
}
$$(".snav").forEach(b=>b.addEventListener("click",()=>openSettingsTab(b.dataset.tab)));

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
function mergeTransformResult(seed, res){ const tb=activeTab(); if(!tb)return; const byLabel={}; tb.graph.nodes.forEach(n=>byLabel[n.label.toLowerCase()]=n.id);
  (res.entities||[]).forEach(e=>{ const key=(e.label||"").toLowerCase(); if(!key)return; if(!byLabel[key]){ const nid="tf-"+Math.abs(hashStr(key+e.kind)); byLabel[key]=nid; tb.graph.nodes.push({id:nid,kind:e.kind||"unknown",label:e.label,risk:0.3,band:"low",attributes:e.attributes||{},tags:["transform"],sources:["transform"]}); } });
  const freshIds=(res.entities||[]).map(e=>byLabel[(e.label||"").toLowerCase()]).filter(Boolean);
  (res.relationships||[]).forEach(r=>{ const s=byLabel[(r.source||"").toLowerCase()]||seed.id, tg=byLabel[(r.target||"").toLowerCase()]; if(s&&tg) tb.graph.edges.push({source:s,target:tg,type:r.type||"related",conf:r.confidence||0.5}); });
  renderGraph(); renderGraphFilters(); flashFresh(freshIds); selectNode(seed.id);
}
function flashFresh(ids){ if(!cy||!ids||!ids.length)return; setTimeout(()=>{ ids.forEach(id=>{ const e=cy.$id(id); if(e&&e.length){ e.addClass("fresh"); setTimeout(()=>e.removeClass("fresh"),1800); } }); },100); }

// ---------- intelligence view ----------
function renderIntelligence(){ const t=activeTab();
  const top=$("#intelTop"), acts=$("#intelActions");
  if(top){ top.innerHTML=""; const nodes=t?[...t.graph.nodes].sort((a,b)=>b.risk-a.risk).slice(0,12):[]; if(!nodes.length)top.innerHTML='<div class="empty">—</div>';
    nodes.forEach(n=>{ const li=el("div","li"); const l=el("div","l"); const d=el("span","kdot"); d.style.background=kColor(n.kind); l.appendChild(d); l.appendChild(el("span","label",n.label)); li.appendChild(l); li.appendChild(el("span","band "+(n.band||bandOf(n.risk)),(n.band||bandOf(n.risk)))); li.addEventListener("click",()=>focusEntity(n.id,true)); top.appendChild(li); }); }
  if(acts){ const risk=t&&t.graph.meta&&t.graph.meta.risk; const items=[...new Set((risk&&risk.assessments||[]).filter(a=>a.recommended_action&&a.recommended_action!=="monitor").map(a=>a.recommended_action))];
    acts.innerHTML=""; if(!items.length)acts.innerHTML='<div class="empty">Run an analysis first.</div>'; items.slice(0,12).forEach(a=>{ const li=el("div","li"); li.appendChild(el("span","label","▸ "+a)); acts.appendChild(li); }); }
}
async function generateIntelligence(){ const t=activeTab(); if(!t||!t.graph.nodes.length){toast("Open a project with a graph","err");return;}
  const b=$("#intelBrief"); b.innerHTML='<div class="empty">✦ synthesizing intelligence…</div>'; setSync("busy","intel");
  try{ const res=await api("/api/ask",{method:"POST",body:{question:"Produce a full intelligence brief for this dataset: the picture it shows, the strongest leads, the biggest risks, and prioritized next actions. Convert data into decision-ready intelligence.",domain:t.project.domain,provider:state.provider,graph:{nodes:t.graph.nodes,edges:t.graph.edges}}});
    let h=`<p>${esc(res.answer||"(no answer)")}</p>`;
    const arr=(title,items)=>{ if(items&&items.length){ h+=`<h3>${title}</h3><ul>`+items.map(x=>`<li>${esc(typeof x==="string"?x:JSON.stringify(x))}</li>`).join("")+"</ul>"; } };
    arr("Key points",res.key_points); arr("Recommended actions",res.recommended_actions);
    b.innerHTML=h; setSync("ok","complete"); pushNotif("ai","Intelligence brief generated"); renderIntelligence();
  }catch(e){ b.innerHTML='<div class="empty">error: '+esc(e.message)+'</div>'; setSync("err","failed"); }
}
$("#btnGenIntel")&&$("#btnGenIntel").addEventListener("click",generateIntelligence);
$("#btnAsk3")&&$("#btnAsk3").addEventListener("click",openAsk);

// ---------- keyboard ----------
window.addEventListener("keydown",e=>{ const meta=e.metaKey||e.ctrlKey;
  if(meta&&e.key.toLowerCase()==="k"){e.preventDefault();openPalette();}
  else if(meta&&e.key.toLowerCase()==="r"){e.preventDefault();runModal();}
  else if(meta&&e.key.toLowerCase()==="n"){e.preventDefault();newProjectModal();}
  else if(meta&&e.key==="/"){e.preventDefault();openAsk();}
  else if(e.key==="Escape"){ closePalette(); closeModal(); $("#ctxmenu").hidden=true; $("#notifDrawer").hidden=true; closeAllSelects(); if(linkMode){ linkMode=null; const b=$("#linkmodeBanner"); if(b)b.hidden=true; } } });

// ---------- file helpers ----------
function pickFile(cb){ const inp=$("#filePicker"); inp.removeAttribute("accept"); inp.value=""; inp.onchange=()=>{ const f=inp.files[0]; if(!f)return; const rd=new FileReader(); rd.onload=()=>cb(rd.result); rd.readAsText(f); }; inp.click(); }
let npUploadPath=null;
// Browse a local file, upload its bytes to the server, and return the server-side path.
function browseUpload(cb, accept){ const inp=$("#filePicker"); if(accept)inp.setAttribute("accept",accept); else inp.removeAttribute("accept"); inp.value="";
  inp.onchange=async ()=>{ const f=inp.files[0]; if(!f)return;
    if(MODE!=="http"){ cb("/uploads/"+f.name); toast("Preview: file path simulated","ok"); return; }
    try{ const buf=await f.arrayBuffer(); const r=await fetch("/api/upload?name="+encodeURIComponent(f.name),{method:"POST",headers:{"Authorization":"Bearer "+TOKEN,"Content-Type":"application/octet-stream"},body:buf}); const j=await r.json(); if(!r.ok)throw new Error(j.error||"upload failed"); cb(j.path); toast("Uploaded "+f.name,"ok"); }
    catch(e){ toast(e.message,"err"); } };
  inp.click(); }
function downloadText(name,text){ const b=new Blob([text],{type:"application/json"}); const a=el("a"); a.href=URL.createObjectURL(b); a.download=name; a.click(); URL.revokeObjectURL(a.href); }

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
