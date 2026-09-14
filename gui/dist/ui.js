/* ===== CortexIntel GUI — workspace layer (v2) =====
   Extends app.js with the flowsint-style workspace: entities panel, selection
   bar, floating toolbars, details panel extras, context menus, console/log,
   keyboard shortcuts, paste-to-add, export, display settings, theme. */
(function(){
"use strict";

// ---------- extra icons (lucide-style) ----------
Object.assign(ICONS, {
  panel:'<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/>',
  pointer:'<path d="M4 4l7.5 16 2.3-6.7L20.5 11z"/>',
  link2:'<path d="M9 17H7A5 5 0 017 7h2M15 7h2a5 5 0 010 10h-2M8 12h8"/>',
  route:'<circle cx="6" cy="19" r="3"/><circle cx="18" cy="5" r="3"/><path d="M12 19h4.5a3.5 3.5 0 000-7h-9a3.5 3.5 0 010-7H12"/>',
  maximize:'<path d="M8 3H5a2 2 0 00-2 2v3M21 8V5a2 2 0 00-2-2h-3M3 16v3a2 2 0 002 2h3M16 21h3a2 2 0 002-2v-3"/>',
  refresh:'<path d="M21 12a9 9 0 11-2.6-6.4M21 3v6h-6"/>',
  filter:'<path d="M3 5h18l-7 8v6l-4 2v-8z"/>',
  sliders:'<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M2 14h4M10 8h4M18 16h4"/>',
  download:'<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/>',
  upload:'<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12"/>',
  rotate:'<path d="M3 12a9 9 0 019-9 9.75 9.75 0 016.74 2.74L21 8M21 3v5h-5M21 12a9 9 0 01-9 9 9.75 9.75 0 01-6.74-2.74L3 16M3 21v-5h5"/>',
  map:'<path d="M3 6l6-3 6 3 6-3v15l-6 3-6-3-6 3z"/><path d="M9 3v15M15 6v15"/>',
  terminal:'<path d="M4 17l6-5-6-5M12 19h8"/>',
  eraser:'<path d="M7 21h10M20 12L11 3 3 11l7 7h4z"/>',
  chevup:'<path d="M18 15l-6-6-6 6"/>', chevdown:'<path d="M6 9l6 6 6-6"/>', chevright:'<path d="M9 6l6 6-6 6"/>',
  sun:'<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  moon:'<path d="M21 12.8A9 9 0 1111.2 3a7 7 0 009.8 9.8z"/>',
  flag:'<path d="M4 22V4s1-1 4-1 5 2 8 2 4-1 4-1v11s-1 1-4 1-5-2-8-2-4 1-4 1"/>',
  pin:'<path d="M12 17v5M9 3h6l-1 7 3 3H7l3-3z"/>',
  merge:'<path d="M7 3v6a4 4 0 004 4h2a4 4 0 004-4V3M12 13v8M9 18l3 3 3-3"/>',
  eyeoff:'<path d="M17.9 17.9A10 10 0 0112 20C5 20 2 12 2 12a18 18 0 015.1-5.9M9.9 4.2A9 9 0 0112 4c7 0 10 8 10 8a18 18 0 01-2.2 3.2M14.1 14.1a3 3 0 11-4.2-4.2M2 2l20 20"/>',
  eye:'<path d="M2 12s3-8 10-8 10 8 10 8-3 8-10 8-10-8-10-8z"/><circle cx="12" cy="12" r="3"/>',
  users:'<circle cx="9" cy="8" r="3.5"/><path d="M2 20c0-3.5 3.2-5.5 7-5.5s7 2 7 5.5M16 4.5a3.5 3.5 0 010 7M22 20c0-2.6-1.7-4.4-4-5"/>',
  list:'<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
  check:'<path d="M20 6L9 17l-5-5"/>',
  info:'<circle cx="12" cy="12" r="9"/><path d="M12 16v-4M12 8h.01"/>',
  keyboard:'<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 13h.01M18 13h.01M8 17h8"/>',
  target:'<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="5"/><circle cx="12" cy="12" r="1"/>',
  expand:'<circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.2 2.2M16.2 16.2l2.2 2.2M5.6 18.4l2.2-2.2M16.2 7.8l2.2-2.2"/>',
  edit:'<path d="M12 20h9M16.5 3.5a2.1 2.1 0 013 3L7 19l-4 1 1-4z"/>',
  bolt:'<path d="M13 2L3 14h9l-1 8 10-12h-9z"/>',
  x:'<path d="M18 6L6 18M6 6l12 12"/>',
  arrowlr:'<path d="M8 3L4 7l4 4M4 7h16M16 21l4-4-4-4M20 17H4"/>',
  scan:'<path d="M3 7V5a2 2 0 012-2h2M17 3h2a2 2 0 012 2v2M21 17v2a2 2 0 01-2 2h-2M7 21H5a2 2 0 01-2-2v-2M7 12h10"/>',
});
const I = n => svg(n);
const setIcon = (sel,name)=>{ const e=$(sel); if(e){ const b=e.querySelector(".badge"); e.innerHTML=I(name); if(b) e.appendChild(b); } };

// ---------- tooltips ----------
const tip = el("div","tooltip"); document.body.appendChild(tip);
let tipT=null;
document.addEventListener("mouseover", e=>{ const t=e.target.closest&&e.target.closest("[data-tip]"); if(!t) return; clearTimeout(tipT);
  tipT=setTimeout(()=>{ tip.innerHTML=esc(t.dataset.tip).replace(/\(([^)]+)\)$/,(m,k)=>` <kbd>${esc(k)}</kbd>`); tip.classList.add("show"); const r=t.getBoundingClientRect(); tip.style.left=Math.max(6,Math.min(window.innerWidth-tip.offsetWidth-6, r.left+r.width/2-tip.offsetWidth/2))+"px";
    let top=r.bottom+6; if(top+tip.offsetHeight>window.innerHeight-4) top=r.top-tip.offsetHeight-6; tip.style.top=top+"px"; },350); });
document.addEventListener("mouseout", e=>{ if(e.target.closest&&e.target.closest("[data-tip]")){ clearTimeout(tipT); tip.classList.remove("show"); } });
document.addEventListener("mousedown", ()=>{ clearTimeout(tipT); tip.classList.remove("show"); }, true);

// ---------- theme ----------
function applyTheme(t){ document.documentElement.setAttribute("data-theme", t); try{ localStorage.setItem("cortex_theme", t); }catch(e){} setIcon("#btnTheme", t==="light"?"moon":"sun"); if(cy){ applyGraphStyle(); renderGraph(); } }
applyTheme((()=>{ try{ return localStorage.getItem("cortex_theme")||"dark"; }catch(e){ return "dark"; } })());
$("#btnTheme").addEventListener("click", ()=>applyTheme(isLight()?"dark":"light"));

// ---------- console / event log ----------
const LOG=[]; const LOG_MAX=400;
function log(level,msg,meta){ const e={t:Date.now(),level,msg:String(msg),meta}; LOG.push(e); if(LOG.length>LOG_MAX) LOG.shift();
  const w=$("#consoleLog"); if(w){ const r=el("div","log-row"); const d=new Date(e.t); const ts=d.toTimeString().slice(0,8);
    r.innerHTML=`<span class="lt">${ts}</span><span class="ll ${level}">${{info:"info",ok:"done",warn:"warn",err:"error"}[level]||level}</span><span class="lm">${esc(e.msg)}</span>`; w.appendChild(r); w.scrollTop=w.scrollHeight; }
  const last=$("#consoleLast"); if(last) last.textContent="· "+e.msg.slice(0,90); }
function consoleOpen(open){ const c=$("#console"); if(!c) return; const o=open===undefined?!c.classList.contains("open"):open; c.classList.toggle("open",o); setIcon("#consoleToggle", o?"chevdown":"chevup"); if(o){ const w=$("#consoleLog"); if(w) w.scrollTop=w.scrollHeight; } if(cy) setTimeout(()=>cy.resize(),200); }
$("#consoleHead").addEventListener("click", e=>{ if(e.target.closest(".icon-btn")) return; consoleOpen(); });
$("#consoleToggle").addEventListener("click", ()=>consoleOpen());
$("#consoleClear").addEventListener("click", ()=>{ LOG.length=0; $("#consoleLog").innerHTML=""; $("#consoleLast").textContent=""; });
$("#sbConsole").addEventListener("click", ()=>{ if(currentView!=="graph") showView("graph"); consoleOpen(true); });
$("#syncStatus").addEventListener("click", ()=>{ if(currentView!=="graph") showView("graph"); consoleOpen(true); });
// hook toast + sync into the log so the console is the single source of truth
const _toast=window.toast; window.toast=function(m,k){ log(k==="err"?"err":k==="ok"?"ok":"info", m); return _toast(m,k); };
const _setSync=window.setSync; window.setSync=function(k,l){ if(k==="busy") log("info", (t2("log.running"))+": "+l); else if(k==="err") log("err", (t2("log.failed"))+": "+l); return _setSync(k,l); };
// runJob: verbose progress so the operator can see the backend working
const _runJob=window.runJob;
window.runJob=async function(kind,payload){ const started=Date.now(); log("info",`job:${kind} → ${payload&&payload.provider?("provider="+payload.provider+" "):""}${payload&&payload.question?("q=\""+String(payload.question).slice(0,60)+"\""):""}`);
  if(MODE!=="http"){ return _runJob(kind,payload); }
  const {job_id}=await api("/api/jobs",{method:"POST",body:{kind,payload}}); log("info",`job ${job_id.slice(0,12)} queued`);
  let lastLog=0; let ticks=0;
  for(;;){ await new Promise(r=>setTimeout(r,1300)); ticks++;
    let s; try{ s=await api("/api/jobs/status?id="+encodeURIComponent(job_id)); }catch(e){ if(Date.now()-started>600000) throw e; log("warn","poll failed, retrying…"); continue; }
    if(Array.isArray(s.log)){ for(let i=lastLog;i<s.log.length;i++){ const line=s.log[i]; const lv=/error|fail|✗/i.test(line)?"err":/done|ok|✓|completed/i.test(line)?"ok":/warn|retry|fallback/i.test(line)?"warn":"info"; log(lv, line); } lastLog=s.log.length; }
    else if(ticks%4===0) log("info",`${kind}: ${t2("log.polling")} (${Math.round((Date.now()-started)/1000)}s)`);
    if(s.status==="done"){ log("ok",`${kind} ${t2("log.done").toLowerCase()} · ${((Date.now()-started)/1000).toFixed(1)}s`+(s.result&&s.result._meta?` · ${s.result._meta.provider||""} ${s.result._meta.model||""}`:"")); return s.result; }
    if(s.status==="error"){ log("err",`${kind}: ${s.error||"job failed"}`); throw new Error(s.error||"job failed"); }
  }
};
log("info","CortexIntel workspace ready");

// ---------- breadcrumbs / status bar ----------
function updateCrumbs(){ const t=activeTab(); const n=$("#crumbProjectName"); if(n) n.textContent=t?t.project.name:"—"; const sb=$("#sbProject b"); if(sb) sb.textContent=t?`${t.project.name} · ${t.project.domain}`:"—";
  const vn=$("#crumbViewName"); if(vn){ const li=$(`.nav li[data-view="${currentView}"] span:last-child`); vn.textContent=li?li.textContent:currentView; }
  const pp=$("#sbProvider"); if(pp) pp.textContent="provider: "+(state.provider||"auto"); }
const _showView=window.showView; window.showView=function(name){ _showView(name); updateCrumbs(); if(name==="graph"){ setTimeout(()=>{ renderNodesPanel(); },50); } };
const _renderTabs=window.renderTabs; window.renderTabs=function(){ _renderTabs(); updateCrumbs(); };
function dropdown(x,y,items){ closeDropdown(); const d=el("div","dropdown"); d.id="ddMenu";
  items.forEach(it=>{ if(it.sep){ d.appendChild(el("div","dd-sep")); return; } if(it.head){ d.appendChild(el("div","dd-head",it.head)); return; }
    const r=el("div","dd-item"+(it.sel?" sel":"")); r.innerHTML=(it.icon?I(it.icon):"")+`<span>${esc(it.label)}</span>`+(it.sub?`<span class="dd-sub">${esc(it.sub)}</span>`:""); r.addEventListener("click",e=>{ e.stopPropagation(); closeDropdown(); it.run&&it.run(); }); d.appendChild(r); });
  document.body.appendChild(d); d.style.left=Math.min(x,window.innerWidth-d.offsetWidth-8)+"px"; d.style.top=Math.min(y,window.innerHeight-d.offsetHeight-8)+"px"; }
function closeDropdown(){ const d=$("#ddMenu"); if(d) d.remove(); }
window.addEventListener("click", closeDropdown);
$("#crumbProject").addEventListener("click", async e=>{ e.stopPropagation(); const r=e.currentTarget.getBoundingClientRect(); let projs=[]; try{ projs=await api("/api/projects"); }catch(err){}
  const items=[{head:t2("crumb.project")}]; const cur=activeTab();
  projs.forEach(p=>items.push({label:p.name, sub:p.domain, icon:"entities", sel:cur&&cur.project.id===p.id, run:()=>openProject(p.id)}));
  items.push({sep:true},{label:t2("btn.newProject"),icon:"plus",run:newProjectModal},{label:t2("launcher.import"),icon:"upload",run:importProjectFlow},{label:"Export project",icon:"download",run:exportActiveProject},{label:"All projects…",icon:"list",run:showLauncher});
  dropdown(r.left,r.bottom+4,items); });
$("#crumbView").addEventListener("click", e=>{ e.stopPropagation(); const r=e.currentTarget.getBoundingClientRect();
  const items=$$(".nav li[data-view]").map(li=>({label:li.querySelector("span:last-child").textContent, icon:NAVICON[li.dataset.view]||"dashboard", sel:li.dataset.view===currentView, run:()=>li.click()}));
  dropdown(r.left,r.bottom+4,items); });
$("#btnDashGraph").addEventListener("click", ()=>showView("graph"));
$("#btnDashRun").addEventListener("click", ()=>runModal());
$("#btnEntAdd").addEventListener("click", ()=>addEntityModal());

// ---------- toolbar icons ----------
function applyToolbarIcons(){
  setIcon("#btnNodesPanel","panel"); setIcon("#btnSelectMode","pointer"); setIcon("#btnAddEntity","plus"); setIcon("#btnConnect","link2"); setIcon("#btnPath","route");
  setIcon("#zoomIn","zoomin"); setIcon("#zoomOut","zoomout"); setIcon("#btnFit","maximize"); setIcon("#btnRelayout","refresh"); setIcon("#btnFilters","filter"); setIcon("#btnGraphSettings","sliders");
  setIcon("#btnExport","download"); setIcon("#btnReset","rotate"); setIcon("#btnAsk2","spark"); setIcon("#zoomFit","maximize");
  setIcon("#npTabList .ni","list"); setIcon("#npTabAdd .ni","plus"); setIcon("#npSelAi","spark"); setIcon("#npSelDelete","trash"); setIcon("#npSelRun .ni","bolt"); setIcon("#npSelClear","x"); setIcon("#npFilterBtn","filter"); setIcon(".np-search .ni","search");
  setIcon("#consoleHead .ni","terminal"); setIcon("#consoleClear","eraser"); setIcon("#consoleToggle","chevup"); setIcon("#sbConsole .ni","terminal"); setIcon("#sbShortcuts .ni","keyboard");
  setIcon("#ctxEnrich","bolt"); setIcon("#ctxEdit","edit"); setIcon("#ctxDelete","trash"); setIcon("#ctxClose","x");
  setIcon('#canvasSwitch [data-canvas="graph"] .ni',"graph"); setIcon('#canvasSwitch [data-canvas="map"] .ni',"map");
  const gs=$(".gs-icon"); if(gs) gs.innerHTML=I("search");
  setIcon("#btnNotifications","bell"); const bn=$("#btnNotifications"); if(bn&&!bn.querySelector(".badge")){ const b=el("span","badge","0"); b.id="notifBadge"; b.hidden=true; bn.appendChild(b); }
}
applyToolbarIcons();
const _applyIcons=window.applyIcons; window.applyIcons=function(){ _applyIcons(); applyToolbarIcons(); updateCrumbs(); };

// ---------- graph settings popover ----------
let settingsPop=null;
function toggleSettings(){ if(settingsPop){ settingsPop.remove(); settingsPop=null; return; }
  const p=el("div","popover"); settingsPop=p; p.style.left="12px"; p.style.top="56px";
  const rows=[
    ["Node style", "select", "outlined", [["false","Filled"],["true","Outlined"]]],
    ["Node size", "range", "nodeSize", [16,60,1]],
    ["Size by connections", "bool", "weightBySize"],
    ["Label size", "range", "labelSize", [7,18,1]],
    ["Show labels", "bool", "showLabels"],
    ["Show icons", "bool", "showIcons"],
    ["Link width", "range", "linkWidth", [0.4,4,0.1]],
    ["Color links by target type", "bool", "autoColorLinks"],
    ["Arrows", "bool", "arrows"],
    ["Edge labels", "select", "edgeLabels", [["hover","On hover / select"],["always","Always"]]],
    ["Dotted background", "bool", "dots"],
    ["Minimap", "bool", "minimap"],
    ["Legend", "bool", "legend"],
  ];
  p.innerHTML=`<h5>Display settings <span class="muted" style="float:right;cursor:pointer;margin:0" id="gsReset">reset</span></h5>`;
  rows.forEach(([lbl,type,key,opts])=>{ const r=el("div","prow"); r.appendChild(el("span",null,lbl)); let inp;
    if(type==="bool"){ const l=el("label","switch"); inp=el("input"); inp.type="checkbox"; inp.checked=!!GS[key]; l.appendChild(inp); r.appendChild(l); inp.addEventListener("change",()=>{ GS[key]=inp.checked; onGS(key); }); }
    else if(type==="range"){ const w=el("div"); w.style.cssText="display:flex;align-items:center;gap:6px"; inp=el("input"); inp.type="range"; inp.min=opts[0]; inp.max=opts[1]; inp.step=opts[2]; inp.value=GS[key]; const v=el("span","pv",String(GS[key])); w.appendChild(inp); w.appendChild(v); r.appendChild(w); inp.addEventListener("input",()=>{ GS[key]=parseFloat(inp.value); v.textContent=inp.value; onGS(key); }); }
    else { inp=el("select","mini-select"); opts.forEach(([v,l])=>{ const o=el("option",null,l); o.value=v; if(String(GS[key])===v) o.selected=true; inp.appendChild(o); }); r.appendChild(inp); inp.addEventListener("change",()=>{ GS[key]=inp.value==="true"?true:inp.value==="false"?false:inp.value; onGS(key); }); }
    p.appendChild(r); });
  p.querySelector("#gsReset").addEventListener("click",()=>{ Object.assign(GS,GS_DEFAULT); saveGS(); toggleSettings(); toggleSettings(); onGS("all"); });
  p.addEventListener("click",e=>e.stopPropagation());
  $("#gcanvas").appendChild(p);
}
function onGS(key){ saveGS();
  $("#gcanvas").classList.toggle("dots",!!GS.dots); $("#minimapWrap").hidden=!GS.minimap||canvasMode==="map"; $("#legend").classList.toggle("collapsed",!GS.legend);
  if(["nodeSize","weightBySize","outlined","all"].includes(key)) renderGraph(); else applyGraphStyle(); }
$("#btnGraphSettings").addEventListener("click", e=>{ e.stopPropagation(); toggleSettings(); });
window.addEventListener("click", e=>{ if(settingsPop && !e.target.closest(".popover") && !e.target.closest("#btnGraphSettings")){ settingsPop.remove(); settingsPop=null; } });
onGS("init");

// ---------- toolbar actions ----------
let selectMode=false;
function setSelectMode(on){ selectMode=on; $("#btnSelectMode").classList.toggle("active",on); $("#gcanvas").classList.toggle("select-mode",on); if(cy){ cy.userPanningEnabled(!on); cy.boxSelectionEnabled(true); cy.autoungrabify(on); } }
$("#btnSelectMode").addEventListener("click", ()=>setSelectMode(!selectMode));
$("#btnRelayout").addEventListener("click", ()=>{ if(cy){ runLayout(); log("info","layout: "+$("#graphLayout").value); } });
$("#btnFilters").addEventListener("click", ()=>{ const f=$("#graphFilters"); f.hidden=!f.hidden; $("#btnFilters").classList.toggle("active",!f.hidden); });
$("#gfClose").addEventListener("click", ()=>{ $("#graphFilters").hidden=true; $("#btnFilters").classList.remove("active"); });
$("#btnNodesPanel").addEventListener("click", ()=>toggleNodesPanel());
function toggleNodesPanel(force){ const p=$("#nodesPanel"); const hide=force===undefined?!p.hidden:!force; p.hidden=hide; $("#btnNodesPanel").classList.toggle("active",!hide); if(cy) setTimeout(()=>cy.resize(),50); }
$("#btnNodesPanel").classList.add("active");
$("#btnExport").addEventListener("click", e=>{ e.stopPropagation(); const r=e.currentTarget.getBoundingClientRect();
  dropdown(r.left, r.bottom+6, [{head:"Export"},{label:"PNG (current view)",icon:"download",run:()=>exportPng(false)},{label:"PNG (full graph, 2x)",icon:"download",run:()=>exportPng(true)},{label:"JSON (nodes + edges)",icon:"download",run:exportJson},{label:"CSV (entities)",icon:"download",run:exportCsv},{sep:true},{label:"Export project",icon:"upload",run:exportActiveProject}]); });
function exportPng(full){ if(!cy||!cy.nodes().length){ toast("Nothing to export","err"); return; } const bg=isLight()?"#fafafa":"#181818"; const uri=cy.png({full:full, scale:full?2:window.devicePixelRatio||1, bg}); const a=el("a"); a.href=uri; a.download=(activeTab()?.project.name||"graph").replace(/[^\w-]+/g,"_")+".png"; a.click(); log("ok","exported PNG"); }
function exportJson(){ const t=activeTab(); if(!t) return; const nodes=t.graph.nodes.map(n=>({id:n.id,kind:n.kind,label:n.label,risk:n.risk,attributes:n.attributes,tags:n.tags,sources:n.sources})); downloadText((t.project.name||"graph").replace(/[^\w-]+/g,"_")+".graph.json", JSON.stringify({nodes,edges:t.graph.edges},null,2)); log("ok","exported JSON"); }
function exportCsv(){ const t=activeTab(); if(!t) return; const q=v=>'"'+String(v==null?"":v).replace(/"/g,'""')+'"'; const rows=[["id","kind","label","risk","band","tags","sources"].join(",")]; t.graph.nodes.forEach(n=>rows.push([n.id,n.kind,n.label,(n.risk||0).toFixed(3),n.band||bandOf(n.risk),(n.tags||[]).join("|"),(n.sources||[]).join("|")].map(q).join(","))); const b=new Blob([rows.join("\n")],{type:"text/csv"}); const a=el("a"); a.href=URL.createObjectURL(b); a.download="entities.csv"; a.click(); log("ok","exported CSV"); }
$("#btnEntExport").addEventListener("click", exportCsv);

// ---------- entities panel (virtualized) ----------
const NP={ tab:"list", q:"", kinds:null, rows:[], checked:new Set(), rowH:40 };
function npVisible(){ const t=activeTab(); if(!t) return []; const q=NP.q.toLowerCase(); let rows=t.graph.nodes;
  if(NP.kinds) rows=rows.filter(n=>NP.kinds.has(n.kind));
  if(q) rows=rows.filter(n=>(n.label+" "+n.kind+" "+(n.tags||[]).join(" ")).toLowerCase().includes(q));
  return rows; }
function npRowH(){ const v=parseInt(getComputedStyle(document.documentElement).getPropertyValue("--np-row")); return v>0?v:40; }
function renderNodesPanel(){ const list=$("#npList"); if(!list) return; const t=activeTab(); NP.rowH=npRowH();
  NP.rows=npVisible(); const total=t?t.graph.nodes.length:0; $("#npCount").textContent=NP.rows.length===total?String(total):`${NP.rows.length}/${total}`;
  $("#npSpacer").style.height=(NP.rows.length*NP.rowH)+"px";
  if(!NP.rows.length){ let e=list.querySelector(".np-empty"); if(!e){ e=el("div","np-empty"); list.appendChild(e); } e.textContent=total?"No nodes match your search":"No entities yet — add one or run an analysis"; } else { const e=list.querySelector(".np-empty"); if(e) e.remove(); }
  renderNpRows(); renderNpFilters(); syncNpAll(); }
function renderNpRows(){ const list=$("#npList"); const top=list.scrollTop; const H=list.clientHeight||400; const start=Math.max(0,Math.floor(top/NP.rowH)-5); const end=Math.min(NP.rows.length, Math.ceil((top+H)/NP.rowH)+5);
  list.querySelectorAll(".np-row").forEach(r=>r.remove()); const cur=cy&&cy.$("node:selected").length===1?cy.$("node:selected")[0].id():null; const frag=document.createDocumentFragment();
  for(let i=start;i<end;i++){ const n=NP.rows[i]; const r=el("div","np-row"+(n.id===cur?" current":"")+(NP.checked.has(n.id)?" checked":"")); r.style.top=(i*NP.rowH)+"px"; const c=kColor(n.kind); const band=n.band||bandOf(n.risk);
    r.innerHTML=`<input type="checkbox" ${NP.checked.has(n.id)?"checked":""}/><span class="np-ic" style="background:${c}">${svg2(n.kind)}</span><span class="lbl" title="${esc(n.label)}">${esc(n.label)}</span><span class="risk-dot" style="background:${bandColor(band)}" data-tip="risk ${band}"></span><span class="kind-badge" style="--kc:${c};--kc-soft:${hexA(c,0.18)}">${esc(n.kind)}</span>`;
    const cb=r.querySelector("input"); cb.addEventListener("click",e=>{ e.stopPropagation(); npToggle(n.id, cb.checked, e.shiftKey, i); });
    r.addEventListener("click",()=>{ selectNode(n.id); });
    r.addEventListener("dblclick",()=>{ if(cy){ const e=cy.$id(n.id); if(e.length) cy.animate({center:{eles:e},zoom:1.6,duration:300}); } });
    r.addEventListener("contextmenu",e=>{ e.preventDefault(); openCtxMenu(e.clientX,e.clientY,n.id); });
    frag.appendChild(r); }
  list.appendChild(frag); }
let npLastIdx=null;
function npToggle(id,on,shift,idx){ if(shift&&npLastIdx!=null){ const [a,b]=[Math.min(npLastIdx,idx),Math.max(npLastIdx,idx)]; for(let i=a;i<=b;i++){ const n=NP.rows[i]; if(on) NP.checked.add(n.id); else NP.checked.delete(n.id); } } else { if(on) NP.checked.add(id); else NP.checked.delete(id); }
  npLastIdx=idx; syncSelectionFromPanel(); }
function syncSelectionFromPanel(){ if(cy){ cy.batch(()=>{ cy.nodes().forEach(n=>{ const want=NP.checked.has(n.id()); if(want!==n.selected()){ if(want) n.select(); else n.unselect(); } }); }); } renderSelectionBar(); renderNpRows(); syncNpAll(); }
function syncNpAll(){ const all=$("#npAll"); if(!all) return; const vis=NP.rows; all.checked=vis.length>0&&vis.every(n=>NP.checked.has(n.id)); all.indeterminate=!all.checked&&vis.some(n=>NP.checked.has(n.id)); }
$("#npAll").addEventListener("change", e=>{ NP.rows.forEach(n=>{ if(e.target.checked) NP.checked.add(n.id); else NP.checked.delete(n.id); }); syncSelectionFromPanel(); });
$("#npList").addEventListener("scroll", ()=>{ requestAnimationFrame(renderNpRows); });
$("#npSearch").addEventListener("input", e=>{ NP.q=e.target.value.trim(); renderNodesPanel(); const gf=$("#graphFilter"); if(gf){ gf.value=NP.q; gf.dispatchEvent(new Event("input")); } });
$("#npFilterBtn").addEventListener("click", ()=>{ const f=$("#npFilters"); f.hidden=!f.hidden; $("#npFilterBtn").classList.toggle("active",!f.hidden); });
function renderNpFilters(){ const w=$("#npFilters"); if(!w) return; const t=activeTab(); const counts={}; (t?.graph.nodes||[]).forEach(n=>counts[n.kind]=(counts[n.kind]||0)+1); w.innerHTML="";
  Object.entries(counts).sort((a,b)=>b[1]-a[1]).forEach(([k,c])=>{ const chip=el("div","gf-kind"+((NP.kinds&&!NP.kinds.has(k))?" off":"")); chip.innerHTML=`<span class="kdot" style="background:${kColor(k)}"></span>${esc(k)} <span class="muted" style="margin:0">${c}</span>`; chip.addEventListener("click",()=>{ if(!NP.kinds) NP.kinds=new Set(Object.keys(counts)); if(NP.kinds.has(k)) NP.kinds.delete(k); else NP.kinds.add(k); if(NP.kinds.size===Object.keys(counts).length) NP.kinds=null; renderNodesPanel(); }); w.appendChild(chip); }); }
$$(".np-tab").forEach(b=>b.addEventListener("click",()=>{ NP.tab=b.dataset.nptab; $$(".np-tab").forEach(x=>x.classList.toggle("active",x===b)); $("#npListWrap").hidden=NP.tab!=="list"; $("#npAdd").hidden=NP.tab!=="add"; if(NP.tab==="add") renderKindGrid(); }));
// quick add
const QUICK_KINDS=["person","organization","account","email","phone","username","domain","url","ip","device","wallet","payment","location","media","document","malware","incident","vulnerability","vehicle","event","group","case"];
let npKind="person";
function renderKindGrid(){ const g=$("#npKindGrid"); g.innerHTML=""; QUICK_KINDS.forEach(k=>{ const o=el("div","kind-opt"+(k===npKind?" sel":"")); o.innerHTML=`<span class="np-ic" style="background:${kColor(k)}">${svg2(k)}</span>${esc(k)}`; o.addEventListener("click",()=>{ npKind=k; renderKindGrid(); $("#npAddLabel").focus(); }); g.appendChild(o); }); }
function quickAdd(kind,label,attrs){ const t=activeTab(); if(!t){ toast("Open a project first","err"); return null; } label=(label||"").trim(); if(!label) return null;
  const dup=t.graph.nodes.find(n=>n.kind===kind&&n.label.toLowerCase()===label.toLowerCase()); if(dup){ toast("Already exists: "+dup.label); selectNode(dup.id); return dup.id; }
  const id="man-"+Math.abs(hashStr(kind+label+String(Date.now())));
  t.graph.nodes.push({ id, kind, label, risk:0.3, band:"low", attributes:attrs||{}, tags:["manual"], sources:["manual"], sensitive:["media","evidence","victim","communication"].includes(kind) });
  log("ok",`added ${kind} "${label}"`); return id; }
function afterAdd(ids, anchorId){ const t=activeTab(); if(!t||!ids.length) return; if(cy&&(t.clusterMode||"none")==="none"&&cy.nodes().length){ const nodes=ids.map(id=>t.graph.nodes.find(n=>n.id===id)).filter(Boolean); const edges=[]; appendToCy(nodes,edges,anchorId||(cy.$(":selected").length?cy.$(":selected")[0].id():null)||cy.nodes()[0].id()); } else renderGraph();
  renderGraphFilters(); renderNodesPanel(); flashFresh(ids); $("#graphEmpty").hidden=true; pushNotif("entity",`${ids.length} entity(ies) added`); if(ids.length===1) setTimeout(()=>selectNode(ids[0]),200); }
$("#npAddBtn").addEventListener("click", ()=>{ const id=quickAdd(npKind,$("#npAddLabel").value); if(id){ $("#npAddLabel").value=""; afterAdd([id]); } });
$("#npAddLabel").addEventListener("keydown", e=>{ if(e.key==="Enter"){ e.preventDefault(); $("#npAddBtn").click(); } });
$("#npAddMore").addEventListener("click", ()=>addEntityModal());
(function(){ const host=$("#npAdd"); if(host && typeof investigateModal==="function"){ const b=el("button","btn ghost block"); b.style.marginTop="6px"; b.innerHTML="✦ Investigar sujeito com IA"; b.addEventListener("click",()=>investigateModal()); host.appendChild(b); } })();
// paste-to-add: detect selectors in clipboard text
function detectSelectors(text){ const out=[]; const seen=new Set(); const push=(kind,v)=>{ const k=kind+":"+v.toLowerCase(); if(seen.has(k)) return; seen.add(k); out.push({kind,label:v}); };
  (text.match(/[\w.+-]+@[\w-]+\.[\w.-]+/g)||[]).forEach(v=>push("email",v));
  (text.match(/https?:\/\/[^\s"'<>]+/g)||[]).forEach(v=>push("url",v));
  (text.match(/\b(?:\d{1,3}\.){3}\d{1,3}\b/g)||[]).forEach(v=>push("ip",v));
  (text.match(/\b(?:0x[a-fA-F0-9]{40}|[13][a-km-zA-HJ-NP-Z1-9]{25,34}|bc1[a-z0-9]{25,60})\b/g)||[]).forEach(v=>push("wallet",v));
  (text.match(/\b[a-fA-F0-9]{32}\b|\b[a-fA-F0-9]{40}\b|\b[a-fA-F0-9]{64}\b/g)||[]).forEach(v=>push("hash",v));
  (text.match(/\+?\d[\d\s().-]{7,}\d/g)||[]).forEach(v=>{ if(v.replace(/\D/g,"").length>=8) push("phone",v.trim()); });
  const noUrl=text.replace(/https?:\/\/[^\s"'<>]+/g," ").replace(/[\w.+-]+@[\w-]+\.[\w.-]+/g," ");
  (noUrl.match(/\b(?:[a-z0-9-]+\.)+(?:com|net|org|io|br|gov|edu|info|onion|co|us|uk|de|fr|ru|cn|xyz|app|dev|me)\b/gi)||[]).forEach(v=>push("domain",v.toLowerCase()));
  return out; }
document.addEventListener("paste", e=>{ if(currentView!=="graph") return; const a=document.activeElement; if(a&&(a.tagName==="INPUT"||a.tagName==="TEXTAREA"||a.isContentEditable)) return;
  const text=(e.clipboardData||window.clipboardData).getData("text"); if(!text) return; const sel=detectSelectors(text); if(!sel.length){ toast("No selectors found in clipboard"); return; }
  const ids=sel.slice(0,50).map(s=>quickAdd(s.kind,s.label)).filter(Boolean); if(ids.length){ afterAdd(ids); toast(`Added ${ids.length} from clipboard`,"ok"); } });

// ---------- selection bar + bulk actions ----------
function selectedIds(){ return cy?cy.$("node:selected").map(n=>n.id()).filter(id=>nodeData(id)&&!nodeData(id).meta):[]; }
function onSelectionChange(){ const ids=selectedIds(); NP.checked=new Set(ids); renderSelectionBar(); renderNpRows(); syncNpAll(); const sb=$("#sbSel"); if(sb) sb.textContent=ids.length>1?`${ids.length} selected`:""; }
function renderSelectionBar(){ const ids=[...NP.checked]; const bar=$("#npSel"); const lst=$("#npSelList"); if(!bar) return; bar.hidden=ids.length<2; lst.hidden=ids.length<2; $("#npSelCount").textContent=ids.length; $("#npSelRunN").textContent=`(${ids.length})`;
  lst.innerHTML=""; ids.slice(0,8).forEach(id=>{ const n=nodeData(id); if(!n) return; const r=el("div","np-selitem"); r.innerHTML=`<span class="kdot" style="width:8px;height:8px;border-radius:50%;background:${kColor(n.kind)}"></span><span class="lbl">${esc(n.label)}</span><span class="kind-badge" style="--kc:${kColor(n.kind)};--kc-soft:${hexA(kColor(n.kind),0.18)}">${esc(n.kind)}</span><span class="x">✕</span>`; r.querySelector(".x").addEventListener("click",()=>{ NP.checked.delete(id); syncSelectionFromPanel(); }); lst.appendChild(r); });
  if(ids.length>8) lst.appendChild(el("div","muted",`+${ids.length-8} more`)); }
$("#npSelClear").addEventListener("click", ()=>{ NP.checked.clear(); syncSelectionFromPanel(); clearFocus&&clearFocus(); });
$("#npSelDelete").addEventListener("click", ()=>removeNodes([...NP.checked]));
$("#npSelAi").addEventListener("click", ()=>{ const labels=[...NP.checked].map(id=>nodeData(id)?.label).filter(Boolean).slice(0,40); openAsk(); $("#askText").value=`Analise as entidades selecionadas e o que as conecta: ${labels.join(", ")}`; });
$("#npSelRun").addEventListener("click", async e=>{ e.stopPropagation(); const ids=[...NP.checked]; if(!ids.length) return; let inst=[]; try{ inst=await api("/api/transforms"); }catch(err){}
  const kinds=new Set(ids.map(id=>nodeData(id)?.kind)); const match=inst.filter(t=>t.enabled&&(!t.input_kinds.length||[...kinds].some(k=>t.input_kinds.includes(k))));
  if(!match.length){ toast("No transforms installed for these types — Settings → Transforms","err"); return; } const r=e.currentTarget.getBoundingClientRect();
  dropdown(r.left,r.bottom+4,[{head:`Run on ${ids.length} selected`}].concat(match.map(t=>({label:t.name,sub:t.runtime,icon:"bolt",run:()=>runTransformBatch(t,ids)})))); });
async function runTransformBatch(t,ids){ log("info",`transform ${t.name} × ${ids.length}`); setSync("busy","transform"); let added=0;
  for(const id of ids){ const n=nodeData(id); if(!n||(t.input_kinds.length&&!t.input_kinds.includes(n.kind))) continue;
    try{ const res=await api("/api/transforms/run",{method:"POST",body:{id:t.id,input:{kind:n.kind,label:n.label,attributes:n.attributes},params:{}}}); if(res.error){ log("warn",`${n.label}: ${res.error}`); continue; } mergeTransformResult(n,res); added+=(res.entities||[]).length; log("ok",`${n.label} → +${(res.entities||[]).length}`); }
    catch(err){ log("err",`${n.label}: ${err.message}`); } }
  setSync("ok","complete"); toast(`${t.name}: +${added} entities`,"ok"); renderNodesPanel(); }
function removeNodes(ids){ const t=activeTab(); if(!t||!ids.length) return; const set=new Set(ids); t.graph.nodes=t.graph.nodes.filter(n=>!set.has(n.id)); t.graph.edges=t.graph.edges.filter(e=>!set.has(e.source)&&!set.has(e.target)); NP.checked.clear(); $("#context").hidden=true; renderGraph(); renderGraphFilters(); renderNodesPanel(); log("ok",`removed ${ids.length} node(s)`); toast(`${ids.length} removed`); }
function mergeNodes(ids){ const t=activeTab(); if(!t||ids.length<2) return; const nodes=ids.map(id=>t.graph.nodes.find(n=>n.id===id)).filter(Boolean); const keep=nodes[0];
  const opts=nodes.map(n=>`<option value="${esc(n.id)}">${esc(n.label)} (${esc(n.kind)})</option>`).join("");
  openModal(`Merge ${nodes.length} nodes`, `<div class="field">Keep as primary<select id="mgKeep" class="select">${opts}</select></div><p class="muted">Edges, tags, sources and attributes of the others are folded into the primary; duplicates removed.</p>`,
    [{label:"Cancel",cls:"ghost",act:closeModal},{label:"Merge",cls:"primary",act:()=>{ const kid=$("#mgKeep").value; const k=t.graph.nodes.find(n=>n.id===kid); const others=nodes.filter(n=>n.id!==kid); const drop=new Set(others.map(n=>n.id));
      others.forEach(o=>{ k.tags=[...new Set([...(k.tags||[]),...(o.tags||[])])]; k.sources=[...new Set([...(k.sources||[]),...(o.sources||[])])]; Object.entries(o.attributes||{}).forEach(([a,v])=>{ if(k.attributes[a]==null) k.attributes[a]=v; }); k.risk=Math.max(k.risk||0,o.risk||0); k.band=bandOf(k.risk); });
      t.graph.edges.forEach(e=>{ if(drop.has(e.source)) e.source=kid; if(drop.has(e.target)) e.target=kid; }); t.graph.edges=t.graph.edges.filter(e=>e.source!==e.target); const seen=new Set(); t.graph.edges=t.graph.edges.filter(e=>{ const key=e.source+"|"+e.target+"|"+e.type; if(seen.has(key)) return false; seen.add(key); return true; });
      t.graph.nodes=t.graph.nodes.filter(n=>!drop.has(n.id)); if(!k.tags.includes("merged")) k.tags.push("merged"); closeModal(); NP.checked.clear(); renderGraph(); renderGraphFilters(); renderNodesPanel(); selectNode(kid); log("ok",`merged ${others.length} into ${k.label}`); toast("Merged","ok"); }}]); }

// ---------- context menus ----------
const FLAGS=[["#ef4444","red"],["#fb923c","orange"],["#facc15","yellow"],["#4ade80","green"],["#5b9cf5","blue"],["#a78bfa","purple"]];
function menu(x,y,items){ const m=$("#ctxmenu"); m.innerHTML="";
  items.forEach(it=>{ if(it.sep){ m.appendChild(el("div","mi sep")); return; } if(it.head){ m.appendChild(el("div","mi head",it.head)); return; }
    const mi=el("div","mi"+(it.danger?" danger":"")); mi.innerHTML=(it.dot?`<span class="flagdot" style="background:${it.dot}"></span>`:it.icon?I(it.icon):"")+`<span>${esc(it.label)}</span>`+(it.kbd?`<kbd>${esc(it.kbd)}</kbd>`:""); mi.addEventListener("click",e=>{ e.stopPropagation(); m.hidden=true; it.run&&it.run(); }); m.appendChild(mi); });
  m.hidden=false; m.style.left=Math.min(x,window.innerWidth-m.offsetWidth-8)+"px"; m.style.top=Math.min(y,window.innerHeight-m.offsetHeight-8)+"px"; }
window.openCtxMenu=async function(x,y,id){ const n=nodeData(id); if(!n) return; const sel=selectedIds(); const multi=sel.length>1&&sel.includes(id);
  if(n.meta){ menu(x,y,[{label:`Expand cluster (${n.members.length})`,icon:"graph",run:()=>expandCluster(id)},{label:"Focus cluster",icon:"maximize",run:()=>{ if(cy) cy.animate({fit:{eles:cy.$id(id),padding:120},duration:300}); }}]); return; }
  const items=[{head:multi?`${sel.length} selected`:n.label.slice(0,40)}];
  if(multi){ items.push({label:"Merge selected…",icon:"merge",run:()=>mergeNodes(sel)},{label:"Ask AI about selection",icon:"spark",run:()=>$("#npSelAi").click()},{label:"Run transform on selection…",icon:"bolt",run:()=>$("#npSelRun").click()},{label:"Select neighbors too",icon:"users",run:()=>{ cy.$("node:selected").closedNeighborhood("node").select(); }},{sep:true},{label:`Remove ${sel.length} nodes`,icon:"trash",danger:true,kbd:"⌫",run:()=>removeNodes(sel)}); menu(x,y,items); return; }
  items.push({label:"Open details",icon:"entities",run:()=>selectNode(id)},{label:"Edit entity…",icon:"edit",kbd:"E",run:()=>editEntityModal(id)},{label:"Expand via AI",icon:"spark",run:()=>askAbout(`Expanda a investigação em torno de "${n.label}" (${n.kind}). Proponha entidades ligadas e próximos leads.`)},{sep:true},
    {label:"Select neighbors",icon:"users",run:()=>{ cy.$id(id).closedNeighborhood("node").select(); }},{label:"Focus neighbors",icon:"target",run:()=>{ if(cy) cy.animate({fit:{eles:cy.$id(id).closedNeighborhood(),padding:80},duration:350}); }},{label:"Isolate neighborhood",icon:"scan",run:()=>isolate(id)},{label:"Neighborhood lens (2 hops)",icon:"graph",run:()=>setGraphMode("neighborhood",{seed:id,hops:2})},{sep:true},
    {label:"Find path from here…",icon:"route",kbd:"P",run:()=>startPath(id)},{label:"Connect to another node…",icon:"link2",kbd:"C",run:()=>startLink(id)},{sep:true},
    {label:n._pinned?"Unpin position":"Pin position",icon:"pin",run:()=>{ n._pinned=!n._pinned; const e=cy.$id(id); if(n._pinned){ e.lock(); e.addClass("pinned"); } else { e.unlock(); e.removeClass("pinned"); } }},
    {label:"Hide node",icon:"eyeoff",kbd:"H",run:()=>{ cy.$id(id).style("display","none"); log("info","hidden: "+n.label); }},
    {label:"Copy label",icon:"copy",run:()=>{ copyToClipboard(n.label); toast("Copied","ok"); }},{head:"Flag"});
  FLAGS.forEach(([c,name])=>items.push({label:name,dot:c,run:()=>setFlag(id,c)})); if(n._flag) items.push({label:"Clear flag",icon:"x",run:()=>setFlag(id,null)});
  items.push({sep:true},{label:"Create alert",icon:"alerts",run:()=>{ pushNotif("alert",`Alert on ${n.label}`); toast("Alert created"); }});
  if(["media","evidence"].includes(n.kind)&&(n.attributes||{}).path) items.push({label:"AI Geolocation (Gemini)",icon:"spark",run:()=>{ cy.$(":selected").unselect(); cy.$id(id).select(); triggerGeminiGeoint(id); }});
  if((activeTab()?.clusterMode||"none")!=="none") items.push({label:"Collapse this cluster",icon:"graph",run:()=>collapseNodeCluster(id)});
  items.push({label:"Remove node",icon:"trash",danger:true,kbd:"⌫",run:()=>removeNode(id)});
  let inst=[]; try{ inst=await api("/api/transforms"); }catch(e){}
  const match=inst.filter(t=>t.enabled&&(!t.input_kinds.length||t.input_kinds.includes(n.kind)));
  if(match.length){ items.push({head:"Transforms"}); match.slice(0,8).forEach(t=>items.push({label:t.name,icon:"bolt",run:()=>{ cy.$(":selected").unselect(); cy.$id(id).select(); runTransformOnSelected(t); }})); }
  menu(x,y,items); };
function setFlag(id,color){ const n=nodeData(id); if(!n) return; n._flag=color||undefined; const e=cy&&cy.$id(id); if(e&&e.length){ e.data("flag",color||undefined); e.toggleClass("flagged",!!color); } if(!$("#context").hidden) selectNode(id); }
function edgeMenu(x,y,edge){ const d=edge.data(); const s=nodeData(d.source), t=nodeData(d.target);
  menu(x,y,[{head:`${s?s.label:"?"} → ${t?t.label:"?"}`},{label:"Edit label…",icon:"edit",run:()=>editEdgeLabel(d.source,d.target)},{label:"Select endpoints",icon:"users",run:()=>{ cy.$(":selected").unselect(); edge.connectedNodes().select(); }},{label:"Reverse direction",icon:"arrowlr",run:()=>{ const tb=activeTab(); const e=tb.graph.edges.find(x=>x.source===d.source&&x.target===d.target); if(e){ [e.source,e.target]=[e.target,e.source]; renderGraph(); } }},{sep:true},{label:"Delete relationship",icon:"trash",danger:true,run:()=>{ const tb=activeTab(); tb.graph.edges=tb.graph.edges.filter(x=>!(x.source===d.source&&x.target===d.target)); renderGraph(); toast("Relationship removed"); }}]); }
function bgMenu(x,y,pos){ menu(x,y,[{label:"Add entity…",icon:"plus",kbd:"A",run:()=>{ $("#npTabAdd").click(); toggleNodesPanel(true); $("#npAddLabel").focus(); }},{label:"Paste selectors",icon:"upload",kbd:"⌘V",run:async()=>{ try{ const txt=await navigator.clipboard.readText(); const sel=detectSelectors(txt); const ids=sel.slice(0,50).map(s=>quickAdd(s.kind,s.label)).filter(Boolean); if(ids.length) afterAdd(ids); else toast("No selectors in clipboard"); }catch(e){ toast("Clipboard unavailable — use ⌘V","err"); } }},{sep:true},
  {label:"Select all",icon:"check",kbd:"⌘A",run:()=>cy.nodes(":visible").select()},{label:"Clear selection",icon:"x",kbd:"Esc",run:()=>{ cy.$(":selected").unselect(); clearFocus(); }},{label:"Show hidden nodes",icon:"eye",run:()=>{ cy.elements().style("display","element"); }},{sep:true},
  {label:"Fit to screen",icon:"maximize",kbd:"F",run:()=>cy.fit(cy.elements(":visible"),50)},{label:"Re-run layout",icon:"refresh",kbd:"L",run:()=>runLayout()},{label:"Export PNG",icon:"download",run:()=>exportPng(false)},{label:"Display settings…",icon:"sliders",run:toggleSettings}]); }
function selectEdge(edge){ cy.$(":selected").unselect(); edge.select(); const d=edge.data(); const s=nodeData(d.source), t=nodeData(d.target); const c=$("#context"); c.hidden=false;
  $("#ctxKind").innerHTML=`<span class="chip">relationship</span>`; $("#ctxName").textContent=`${s?s.label:"?"} → ${t?t.label:"?"}`; $("#ctxRisk").innerHTML=`<span class="band ${d.conf>=0.8?"low":d.conf>=0.5?"medium":"high"}">confidence ${Math.round((d.conf||0.5)*100)}%</span>`;
  const meta=$("#ctxMeta"); meta.innerHTML=""; [["type",d.rtype||d.type],["label",d.elabel||"—"],["source",s?s.label:d.source],["target",t?t.label:d.target],["confidence",(d.conf!=null?d.conf:0.5).toFixed(2)]].forEach(([k,v])=>{ const r=el("div","row"); r.appendChild(el("span","k",k)); r.appendChild(el("span","v",String(v))); meta.appendChild(r); });
  $("#ctxRels").innerHTML=""; $("#ctxRelCount").textContent=""; $("#ctxTags").innerHTML=""; $("#ctxSources").innerHTML=""; $("#ctxTransforms").innerHTML=""; $("#ctxNbTitle").textContent="Endpoints"; renderNeighbors(d.source, d.target);
  $("#ctxComments").innerHTML=""; }
$("#ctxEdit").addEventListener("click", ()=>{ const id=cy&&cy.$("node:selected").length?cy.$("node:selected")[0].id():null; if(id) editEntityModal(id); });
$("#ctxDelete").addEventListener("click", ()=>{ const ids=selectedIds(); if(ids.length) removeNodes(ids); });
$("#ctxEnrich").addEventListener("click", e=>{ e.stopPropagation(); const id=cy&&cy.$("node:selected").length?cy.$("node:selected")[0].id():null; if(!id) return; NP.checked=new Set([id]); $("#npSelRun").dispatchEvent(new MouseEvent("click",{bubbles:true})); });

// ---------- neighbors mini-graph (details panel) ----------
function renderNeighbors(id, otherId){ const cv=$("#ctxNeighbors"); if(!cv) return; const ctx=cv.getContext("2d"); const t=activeTab(); if(!t) return;
  const dpr=window.devicePixelRatio||1; const W=cv.clientWidth||300, H=cv.clientHeight||190; if(cv.width!==W*dpr||cv.height!==H*dpr){ cv.width=W*dpr; cv.height=H*dpr; } ctx.setTransform(dpr,0,0,dpr,0,0); ctx.clearRect(0,0,W,H);
  const center=nodeData(id); if(!center) return; const nbIds=otherId?[otherId]:[...new Set(t.graph.edges.filter(e=>e.source===id||e.target===id).map(e=>e.source===id?e.target:e.source))];
  const nbs=nbIds.map(x=>nodeData(x)).filter(Boolean); const cx=W/2, cy0=H/2; const light=isLight();
  const items=[]; const N=nbs.length; const rings=N<=12?1:N<=40?2:3; const perRing=Math.ceil(N/rings);
  nbs.forEach((n,i)=>{ const ring=Math.floor(i/perRing); const idx=i%perRing; const cnt=Math.min(perRing,N-ring*perRing); const r=(Math.min(W,H)/2-22)*((ring+1)/rings)*(rings===1?0.95:1); const a=(idx/cnt)*Math.PI*2-Math.PI/2; items.push({n,x:cx+Math.cos(a)*r,y:cy0+Math.sin(a)*r,r:N>40?4:N>12?6:9}); });
  ctx.lineWidth=1; ctx.strokeStyle=light?"rgba(0,0,0,0.15)":"rgba(255,255,255,0.14)"; items.forEach(it=>{ ctx.beginPath(); ctx.moveTo(cx,cy0); ctx.lineTo(it.x,it.y); ctx.stroke(); });
  items.forEach(it=>{ ctx.beginPath(); ctx.arc(it.x,it.y,it.r,0,7); ctx.fillStyle=kColor(it.n.kind); ctx.fill(); if(N<=12){ ctx.font="10px Inter, sans-serif"; ctx.fillStyle=light?"#333":"#ccc"; ctx.textAlign="center"; ctx.fillText(it.n.label.length>16?it.n.label.slice(0,15)+"…":it.n.label, it.x, it.y+it.r+11); } });
  ctx.beginPath(); ctx.arc(cx,cy0,13,0,7); ctx.fillStyle=kColor(center.kind); ctx.fill(); ctx.lineWidth=2; ctx.strokeStyle=cssVar("--accent","#e8673d"); ctx.stroke();
  const img=new Image(); img.onload=()=>{ try{ ctx.drawImage(img,cx-7,cy0-7,14,14); }catch(e){} }; img.src=nodeIcon(center.kind,"#fff");
  if(!N){ ctx.font="11px Inter, sans-serif"; ctx.fillStyle=light?"#777":"#777"; ctx.textAlign="center"; ctx.fillText("no neighbors", cx, cy0+32); }
  cv._items=items; cv.onclick=e=>{ const r=cv.getBoundingClientRect(); const mx=e.clientX-r.left, my=e.clientY-r.top; const hit=(cv._items||[]).find(it=>Math.hypot(it.x-mx,it.y-my)<=it.r+4); if(hit) selectNode(hit.n.id); };
  cv.onmousemove=e=>{ const r=cv.getBoundingClientRect(); const mx=e.clientX-r.left, my=e.clientY-r.top; const hit=(cv._items||[]).find(it=>Math.hypot(it.x-mx,it.y-my)<=it.r+4); cv.style.cursor=hit?"pointer":"default"; cv.title=hit?hit.n.label+" · "+hit.n.kind:""; }; }

// ---------- hover tooltip on canvas ----------
function bindCanvasTip(){ if(!cy||cy._tipBound) return; cy._tipBound=true; const tp=$("#graphTip");
  cy.on("mouseover","node",ev=>{ const n=nodeData(ev.target.id()); if(!n) return; const deg=ev.target.degree(false); const band=n.band||bandOf(n.risk);
    tp.innerHTML=`<div class="gt-k">${esc(n.kind)}${n.meta?" · cluster":""}</div><div class="gt-l">${esc(n.label)}</div><div class="gt-m"><span class="band ${band}">${band}</span><span>${deg} links</span>${(n.tags||[]).slice(0,3).map(x=>`<span class="chip">${esc(x)}</span>`).join("")}</div>`; tp.hidden=false; });
  cy.on("mousemove",ev=>{ if(tp.hidden) return; const oe=ev.originalEvent; const r=$("#gcanvas").getBoundingClientRect(); let x=oe.clientX-r.left+14, y=oe.clientY-r.top+14; if(x+tp.offsetWidth>r.width-8) x=oe.clientX-r.left-tp.offsetWidth-10; if(y+tp.offsetHeight>r.height-8) y=oe.clientY-r.top-tp.offsetHeight-10; tp.style.left=x+"px"; tp.style.top=y+"px"; });
  cy.on("mouseout","node",()=>{ tp.hidden=true; }); cy.on("grab tap zoom pan",()=>{ tp.hidden=true; }); }

// ---------- legend (toggle kinds) ----------
window.renderLegend=function(){ const t=activeTab(); const nodes=(t?.graph.nodes||[]); const counts={}; nodes.forEach(n=>counts[n.kind]=(counts[n.kind]||0)+1);
  const lg=$("#legend"); lg.innerHTML=""; if(!nodes.length) return; const title=el("div","lg-title"); title.innerHTML=`<span>Types</span><span style="cursor:pointer" id="lgToggle">${GS.legend?"−":"+"}</span>`; lg.appendChild(title); lg.classList.toggle("collapsed",!GS.legend);
  title.querySelector("#lgToggle").addEventListener("click",()=>{ GS.legend=!GS.legend; saveGS(); renderLegend(); });
  Object.entries(counts).sort((a,b)=>b[1]-a[1]).forEach(([k,c])=>{ const x=el("div","lg"+((gfActiveKinds&&!gfActiveKinds.has(k))?" off":"")); x.innerHTML=`<span class="kdot" style="background:${kColor(k)}"></span><span>${esc(k)}</span><b>${c}</b>`; x.dataset.tip="Click to toggle · Alt-click to solo"; x.addEventListener("click",e=>{ const kinds=Object.keys(counts); if(e.altKey){ gfActiveKinds=new Set([k]); } else { if(!gfActiveKinds) gfActiveKinds=new Set(kinds); if(gfActiveKinds.has(k)) gfActiveKinds.delete(k); else gfActiveKinds.add(k); if(gfActiveKinds.size===kinds.length) gfActiveKinds=null; } applyFilters(); renderGraphFilters(); renderLegend(); }); lg.appendChild(x); }); };

// ---------- graph rendered hook ----------
function onGraphRendered(){ bindCanvasTip(); renderNodesPanel(); onSelectionChange(); const t=activeTab(); if(t&&cy){ t.graph.nodes.forEach(n=>{ if(n._pinned){ const e=cy.$id(n.id); if(e.length){ e.lock(); e.addClass("pinned"); } } }); } }

// ---------- keyboard shortcuts ----------
function inInput(){ const a=document.activeElement; return a&&(a.tagName==="INPUT"||a.tagName==="TEXTAREA"||a.tagName==="SELECT"||a.isContentEditable); }
window.addEventListener("keydown", e=>{ const meta=e.metaKey||e.ctrlKey;
  if(meta&&e.key.toLowerCase()==="d"){ e.preventDefault(); if(currentView!=="graph") showView("graph"); consoleOpen(); return; }
  if(meta&&e.key.toLowerCase()==="b"&&currentView==="graph"){ e.preventDefault(); toggleNodesPanel(); return; }
  if(inInput()) return;
  if(e.key==="?"){ shortcutsModal(); return; }
  if(currentView!=="graph"||!cy) return;
  if(meta&&e.key.toLowerCase()==="a"){ e.preventDefault(); cy.nodes(":visible").select(); return; }
  if(e.key==="Escape"){ cy.$(":selected").unselect(); clearFocus(); $("#context").hidden=true; setSelectMode(false); const f=$("#graphFilters"); if(f) f.hidden=true; return; }
  if((e.key==="Delete"||e.key==="Backspace")){ const ids=selectedIds(); if(ids.length){ e.preventDefault(); if(ids.length>3){ openModal(`Remove ${ids.length} nodes?`,`<p class="muted">This removes them from the working graph (not from the source data).</p>`,[{label:"Cancel",cls:"ghost",act:closeModal},{label:"Remove",cls:"primary",act:()=>{ closeModal(); removeNodes(ids); }}]); } else removeNodes(ids); } return; }
  const k=e.key.toLowerCase();
  if(k==="f"){ cy.fit(cy.elements(":visible"),50); } else if(k==="l"){ runLayout(); } else if(k==="a"){ $("#npTabAdd").click(); toggleNodesPanel(true); $("#npAddLabel").focus(); e.preventDefault(); }
  else if(k==="c"){ $("#btnConnect").click(); } else if(k==="p"){ $("#btnPath").click(); } else if(k==="e"){ const id=selectedIds()[0]; if(id) editEntityModal(id); }
  else if(k==="h"){ selectedIds().forEach(id=>cy.$id(id).style("display","none")); } else if(k==="s"&&!e.repeat){ setSelectMode(true); }
  else if(e.key==="+"||e.key==="="){ $("#zoomIn").click(); } else if(e.key==="-"){ $("#zoomOut").click(); }
  else if(k==="m"&&selectedIds().length>1){ mergeNodes(selectedIds()); } });
window.addEventListener("keyup", e=>{ if(e.key.toLowerCase()==="s"&&!inInput()&&selectMode&&!e.metaKey) setSelectMode(false); });
function shortcutsModal(){ const rows=[["⌘K","Command palette"],["⌘/","Ask AI"],["⌘R","Run analysis"],["⌘D","Console"],["⌘B","Toggle entities panel"],["⌘A","Select all nodes"],["⌘V","Paste selectors → entities"],["Shift/⌘ click","Multi-select"],["Shift drag","Box select"],["S (hold)","Selection mode"],["A","Add entity"],["C","Connect"],["P","Find path"],["E","Edit selected"],["H","Hide selected"],["M","Merge selected"],["F","Fit"],["L","Re-layout"],["+ / −","Zoom"],["⌫","Remove selected"],["Esc","Clear / close"],["?","This help"]];
  openModal("Keyboard shortcuts", `<div class="kv">${rows.map(([k,v])=>`<div class="row"><span class="k">${esc(v)}</span><span class="v"><kbd>${esc(k)}</kbd></span><span></span></div>`).join("")}</div>`, [{label:"Close",cls:"primary",act:closeModal}]); }
$("#sbShortcuts").addEventListener("click", shortcutsModal);
COMMANDS.push(["Keyboard shortcuts","?",shortcutsModal],["Toggle console","⌘D",()=>consoleOpen()],["Toggle theme","",()=>applyTheme(isLight()?"dark":"light")],["Export graph as PNG","",()=>exportPng(false)],["Export entities CSV","",exportCsv],["Merge selected nodes","M",()=>mergeNodes(selectedIds())],["Display settings","",toggleSettings]);

// ---------- resizable side panels ----------
$$(".gpanel-resizer").forEach(h=>{ h.addEventListener("mousedown", e=>{ e.preventDefault(); const side=h.dataset.resize; const panel=h.parentElement; const startX=e.clientX; const startW=panel.offsetWidth;
  const mv=ev=>{ const d=ev.clientX-startX; const w=Math.max(220,Math.min(600, side==="left"?startW+d:startW-d)); panel.style.width=w+"px"; }; const up=()=>{ window.removeEventListener("mousemove",mv); window.removeEventListener("mouseup",up); if(cy) cy.resize(); renderNpRows(); }; window.addEventListener("mousemove",mv); window.addEventListener("mouseup",up); }); });

// ---------- Ask AI routes: entity-aware ----------
function mentionedIds(text,res){ const t=activeTab(); if(!t) return []; const ids=new Set(); const low=(text||"").toLowerCase();
  if(res&&res.focus&&Array.isArray(res.focus.ids)) res.focus.ids.forEach(id=>{ if(t.graph.nodes.some(n=>n.id===id)) ids.add(id); });
  ["labels","entity_labels"].forEach(k=>{ if(res&&res.focus&&Array.isArray(res.focus[k])) res.focus[k].forEach(l=>{ const n=t.graph.nodes.find(x=>x.label.toLowerCase()===String(l).toLowerCase()); if(n) ids.add(n.id); }); });
  t.graph.nodes.forEach(n=>{ if(n.label.length>=3 && low.includes(n.label.toLowerCase())) ids.add(n.id); });
  return [...ids]; }
window.suggestRoutes=function(q,res){ const s=(q||"").toLowerCase(); const out=[]; const has=(...ws)=>ws.some(w=>s.includes(w));
  const text=res?[res.answer,...(res.points||[]),...(res.bullets||[])].filter(Boolean).join("\n"):""; const ids=mentionedIds(text,res);
  if(has("risco","risk","avali","assess","por que","porque","why","decid","priori","hipó","hipo","hypo","ameaç","threat","lavagem","launder","fraud")) out.push({label:"✦ "+t2("route.intel"),run:()=>{showView("intelligence");renderIntelligence();}});
  else if(has("onde","where","local","geo","mapa"," map","país","pais","country","região","regiao","cidade","city")) out.push({label:"🌐 "+t2("route.map"),run:()=>{showView("graph");try{setCanvasMode("map");}catch(e){}}});
  else if(has("quando","when"," tempo","timeline","cronolog","sequ")) out.push({label:"⧗ "+t2("route.timeline"),run:()=>{showView("graph");try{setGraphMode("timeline");}catch(e){}}});
  if(ids.length){ out.push({label:`⬡ ${t2("route.entities")} (${ids.length})`,run:()=>{ window._entIdFilter=new Set(ids); entityFilter="ids"; showView("entities"); renderEntities(); }});
    out.push({label:`◕ ${t2("route.graph")} (${ids.length})`,run:()=>{ applyLocalFocus(ids); setTimeout(()=>{ if(cy){ cy.$(":selected").unselect(); ids.forEach(id=>cy.$id(id).select()); onSelectionChange(); if(ids.length===1) selectNode(ids[0]); } },500); }}); }
  else if(has("quais","quem","who","which","list","liste","entidad","entit","carteira","wallet","conta","account")) out.push({label:"⬡ "+t2("route.entities"),run:()=>{showView("entities");renderEntities();}});
  if(res&&res.focus&&res.focus.action&&res.focus.action!=="none"&&!ids.length) out.push({label:"◕ "+t2("route.graph"),run:()=>applyFocus(res.focus)});
  if(!out.length) out.push({label:"◕ "+t2("route.graph"),run:()=>showView("graph")});
  return out.slice(0,3); };

// ---------- entity registry: sortable, richer table ----------
let entSort={key:"risk",dir:-1};
$$("#entitiesTable th.sortable").forEach(th=>th.addEventListener("click",()=>{ const k=th.dataset.sort; if(entSort.key===k) entSort.dir*=-1; else entSort={key:k,dir:k==="label"||k==="kind"?1:-1}; renderEntities(); }));
window.renderEntities=function(){ const t=activeTab(); const tb=$("#entitiesTable tbody"); if(!tb) return; tb.innerHTML="";
  const g=t?t.graph:{nodes:[],edges:[]}; const m=computeMetrics(g); const dupSet=new Set(m.dupGroups.flat());
  const sc=$("#entSummary"); if(sc){ sc.innerHTML=""; [["Total",m.total,"",()=>{entityFilter="all";renderEntities();}],["High risk",m.highRisk,"crit",()=>{entityFilter="highrisk";renderEntities();}],["Low confidence",m.unresolved,"warn",()=>{entityFilter="lowconf";renderEntities();}],["Likely duplicates",m.duplicates,"warn",()=>{entityFilter="dupes";renderEntities();}]].forEach(([l,v,cls,go])=>{ const d=el("div","dcard "+cls); d.innerHTML=`<div class="dc-v">${v}</div><div class="dc-l">${l}</div>`; d.addEventListener("click",go); sc.appendChild(d); }); }
  const filters=[["all","All",m.total],["highrisk","High risk",m.highRisk],["lowconf","Low confidence",m.unresolved],["dupes","Duplicates",m.duplicates],["missing","Missing evidence",m.missingSource+m.missingMeta],["norel","No relations",m.isolated],["hub","High degree",g.nodes.filter(n=>(m.deg[n.id]||0)>=8).length],["sensitive","Sensitive",m.sensitive],["manual","Manual",g.nodes.filter(n=>n.tags&&n.tags.includes("manual")).length],["review","Needs review",g.nodes.filter(n=>entStatus(t,n)==="review").length]];
  if(entityFilter==="ids"&&window._entIdFilter) filters.unshift(["ids","From answer",window._entIdFilter.size]);
  const fw=$("#entFilters"); if(fw){ fw.innerHTML=""; filters.forEach(([id,label,cnt])=>{ const c=el("button","efilter"+(entityFilter===id?" active":"")); c.innerHTML=`${esc(label)}<span class="cnt">${cnt}</span>`; c.addEventListener("click",()=>{entityFilter=id;renderEntities();}); fw.appendChild(c); }); }
  const q=($("#entSearch")&&$("#entSearch").value||"").trim(); let rows=[...g.nodes];
  const pass=n=>{ const band=n.band||bandOf(n.risk); switch(entityFilter){ case "ids": return window._entIdFilter&&window._entIdFilter.has(n.id); case "highrisk": return band==="high"||band==="critical"; case "lowconf": return (n._rc||0)<0.5; case "dupes": return dupSet.has(n.id); case "missing": return !(n.sources&&n.sources.length)||!Object.keys(n.attributes||{}).length; case "norel": return (n._deg||0)===0; case "hub": return (m.deg[n.id]||0)>=8; case "sensitive": return !!n.sensitive; case "manual": return n.tags&&n.tags.includes("manual"); case "review": return entStatus(t,n)==="review"; default: return true; } };
  rows=rows.filter(pass); if(q) rows=rows.filter(n=>entIntent(q,n,dupSet));
  const val=n=>({label:n.label.toLowerCase(),kind:n.kind,risk:n.risk||0,conf:n._rc||0,qual:n._q||0,deg:m.deg[n.id]||0,src:(n.sources||[]).length})[entSort.key];
  rows.sort((a,b)=>{ const va=val(a), vb=val(b); return (va<vb?-1:va>vb?1:0)*entSort.dir || (b.risk||0)-(a.risk||0); });
  $$("#entitiesTable th.sortable").forEach(th=>{ th.classList.toggle("sorted",th.dataset.sort===entSort.key); const s=th.querySelector(".sort"); if(s) s.remove(); if(th.dataset.sort===entSort.key) th.insertAdjacentHTML("beforeend",`<span class="sort">${entSort.dir>0?"▲":"▼"}</span>`); });
  const shown=rows.slice(0,500); const frag=document.createDocumentFragment();
  shown.forEach(n=>{ const band=n.band||bandOf(n.risk); const tr=el("tr"+(entSel.has(n.id)?"":"")); if(entSel.has(n.id)) tr.className="sel"; const c=kColor(n.kind);
    tr.innerHTML=`<td><input type="checkbox" ${entSel.has(n.id)?"checked":""}></td><td><div class="ent-ent"><span class="eic" style="background:${c}">${svg2(n.kind)}</span><span class="label">${esc(n.label)}</span>${n._flag?`<span class="kdot" style="width:8px;height:8px;border-radius:50%;background:${n._flag};display:inline-block"></span>`:""}</div></td><td>${kindBadge(n.kind)}</td>
      <td><div class="risk-cell"><div class="rc-track"><span style="width:${Math.round((n.risk||0)*100)}%;background:${bandColor(band)}"></span></div><span class="band ${band}">${(n.risk||0).toFixed(2)}</span></div></td><td><span class="score-badge ${scoreCls(n._rc)}">${pct(n._rc)}</span></td><td><span class="qual-badge ${qualityLabel(n._q)}">${qualityLabel(n._q)}</span></td><td>${m.deg[n.id]||0}</td><td>${(n.sources||[]).length}</td><td><span class="st-badge ${entStatus(t,n)}">${entStatus(t,n)==="review"?"needs review":entStatus(t,n)}</span></td><td>${(n.tags||[]).slice(0,3).map(x=>`<span class="chip">${esc(x)}</span>`).join(" ")}</td>`;
    const box=tr.querySelector("input"); box.addEventListener("click",e=>{ e.stopPropagation(); if(box.checked) entSel.add(n.id); else entSel.delete(n.id); tr.classList.toggle("sel",box.checked); updateBulkBar(); });
    tr.addEventListener("click",()=>focusEntity(n.id,false)); tr.addEventListener("contextmenu",e=>{ e.preventDefault(); openCtxMenu(e.clientX,e.clientY,n.id); }); frag.appendChild(tr); });
  tb.appendChild(frag); window._entShownIds=shown.map(n=>n.id);
  const foot=$("#entFoot"); if(foot) foot.innerHTML=`<span>${rows.length} matching · showing ${shown.length}${rows.length>shown.length?" (capped)":""}</span><span>sorted by ${entSort.key} ${entSort.dir>0?"↑":"↓"}</span>`;
  const selAll=$("#entSelAll"); if(selAll) selAll.checked=shown.length>0&&shown.every(n=>entSel.has(n.id)); updateBulkBar(); };

// ---------- models & routing panel (settings) ----------
async function renderModels(){ const tb=$("#modelsTable tbody"); if(!tb) return; let d=null; try{ d=await api("/api/models"); }catch(e){ tb.innerHTML=`<tr><td colspan="9" class="empty">${esc(e.message)}</td></tr>`; return; }
  const f1=x=>Number(x).toFixed(1); const bar=(v,max)=>`<span class="dm-bar"><span style="width:${Math.round(v/max*100)}%;background:${v/max>=0.8?"var(--green)":v/max>=0.55?"var(--amber)":"var(--red)"}"></span></span>${f1(v)}`;
  tb.innerHTML=""; (d.models||[]).sort((a,b)=>b.base_score-a.base_score).forEach(m=>{ const tr=el("tr"); tr.innerHTML=`<td><b>${esc(m.id)}</b></td><td><span class="chip">${esc(m.provider)}</span></td><td><b>${f1(m.base_score)}</b></td><td>${bar(m.reasoning,10)}</td><td>${bar(m.speed,10)}</td><td>${bar(m.cost,10)}</td><td>${m.context_k}k</td><td>${(m.strengths||[]).map(x=>`<span class="chip">${esc(x)}</span>`).join(" ")}</td><td>${m.available?'<span class="tag ok">on</span>':'<span class="tag off">off</span>'}</td>`; tb.appendChild(tr); });
  $("#modelsMeta").textContent=`${(d.models||[]).length} modelos · ${(d.providers||[]).filter(p=>p.available).length} provedores ativos`;
  const r=$("#modelRoutes"); r.innerHTML=""; const tiers={simple:"Simples (classificação, auditoria)",standard:"Padrão (extração, correlação)",complex:"Complexa (risco, investigação, síntese, Ask)"};
  Object.entries(d.routes||{}).forEach(([tier,plan])=>{ const li=el("div","li"); li.style.cursor="default"; li.innerHTML=`<div class="l"><b style="min-width:230px">${esc(tiers[tier]||tier)}</b>${(plan||[]).slice(0,4).map((a,i)=>`<span class="chip" style="${i===0?"border-color:var(--accent);color:var(--accent)":""}" title="${esc(a.reason)}">${i===0?"▶ ":"↳ "}${esc(a.provider)}:${esc(a.model)} <span class="muted" style="margin:0">${a.score>=999?"pin":Number(a.score).toFixed(2)}</span></span>`).join("")}${!(plan||[]).length?'<span class="muted" style="margin:0">nenhum provedor disponível</span>':""}</div>`; r.appendChild(li); });
  const g=$("#governorInfo"); if(g&&d.governor){ const u=d.governor.global||{}; g.innerHTML=[["tokens (in/out)",`${u.tokens_in||0} / ${u.tokens_out||0}`],["custo estimado",`$${(u.usd||0).toFixed(4)}`],["chamadas",u.calls||0],["cache hits",u.cache_hits||0],["orçamento sessão",`${u.budget_tokens||0} tokens · $${(u.budget_usd||0).toFixed(2)}`],["pressão",`${Math.round((d.governor.pressure||0)*100)}%`]].map(([k,v])=>`<div class="row"><span class="k">${k}</span><span class="v">${v}</span><span></span></div>`).join(""); }
  try{ const c=await api("/api/cache/stats"); const w=$("#cacheInfo"); if(w) w.innerHTML=[["entradas",c.entries],["camada semântica",c.semantic],["TTL",c.ttl_h+"h"],["ativo",c.enabled?"sim":"não"]].map(([k,v])=>`<div class="row"><span class="k">${k}</span><span class="v">${esc(String(v))}</span><span></span></div>`).join(""); }catch(e){}
}
$("#btnModelsRefresh").addEventListener("click", renderModels);
$("#btnCacheClear").addEventListener("click", async()=>{ try{ const r=await api("/api/cache/clear",{method:"POST",body:{}}); toast(`Cache limpo (${r.cleared})`,"ok"); renderModels(); }catch(e){ toast(e.message,"err"); } });
const _openSettingsTab=window.openSettingsTab; window.openSettingsTab=function(tab){ _openSettingsTab(tab); if(tab==="providers") renderModels(); };

// ---------- responsive behaviour ----------
const RESP={ narrow:()=>window.innerWidth<=1180, phone:()=>window.innerWidth<=640, userPanel:null };
function applyResponsive(){ const narrow=RESP.narrow();
  // On narrow viewports the side panels overlay the canvas: start with the entities panel
  // closed (unless the user explicitly opened it) so the graph is visible first.
  if(currentView==="graph"){ if(narrow && RESP.userPanel!==true && !$("#nodesPanel").hidden) toggleNodesPanel(false); if(!narrow && RESP.userPanel!==false && $("#nodesPanel").hidden) toggleNodesPanel(true); }
  if(cy) cy.resize(); renderNpRows(); }
const _toggleNodesPanel=toggleNodesPanel;
$("#btnNodesPanel").addEventListener("click", ()=>{ RESP.userPanel=!$("#nodesPanel").hidden; });
let _rzT; window.addEventListener("resize", ()=>{ clearTimeout(_rzT); _rzT=setTimeout(applyResponsive,120); });
window.addEventListener("orientationchange", ()=>setTimeout(applyResponsive,300));
// selecting a node on a narrow screen: show details, hide the entities overlay
const _selectNode=window.selectNode; window.selectNode=function(id){ _selectNode(id); if(RESP.narrow() && !$("#nodesPanel").hidden && RESP.userPanel!==true) toggleNodesPanel(false); };
// closing details on a phone returns to the canvas cleanly
$("#ctxClose").addEventListener("click", ()=>{ if(cy) setTimeout(()=>cy.resize(),50); });
const _showView2=window.showView; window.showView=function(name){ _showView2(name); if(name==="graph") setTimeout(applyResponsive,80); };
// tap on the canvas closes overlay panels on phones
$("#gcanvas").addEventListener("pointerdown", e=>{ if(!RESP.phone()) return; if(e.target.closest(".ftb,.legend,.minimap-wrap,.graph-zoom,.ask-dock,.popover,.graph-filters")) return; if(!$("#nodesPanel").hidden) toggleNodesPanel(false); });
applyResponsive();

// ---------- public hooks ----------
window.UI={ log, consoleOpen, onGraphRendered, onSelectionChange, selectEdge, edgeMenu, bgMenu, renderNeighbors, renderNodesPanel, quickAdd, afterAdd, removeNodes, mergeNodes, selectedIds, updateCrumbs, detectSelectors, exportPng };
updateCrumbs();
})();
