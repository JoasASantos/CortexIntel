<div align="center">

# CortexIntel

**Turn heterogeneous data into correlated, prioritized, auditable intelligence — offline, agnostic, and with an AI decision layer.**

`data → information → intelligence`

Rust CLI + local GUI · deterministic core that runs with **zero external calls** · optional multi-LLM agents · one engine for every vertical.

</div>

---

## Why CortexIntel

Most tools stop at the graph — *"here's the network, you figure it out."* CortexIntel closes the loop: it **potentiates** raw data, **connects** and **correlates** it, and turns it into a **decision** — an assessment in plain language with confidence, evidence, and the next best action.

- **Domain-agnostic.** The same engine serves child-protection / victim ID, cyber / threat-intel, fraud & AML, KYC, logistics, health, commerce, defense, journalism — you pick the lens; the engine is one.
- **Deterministic first.** The full pipeline produces a graph, risk scores, network analysis, anomalies, link predictions and an audit trail **offline** (`--offline`) — no API, no cost. LLM agents *augment*, never gate.
- **Data sovereignty.** It drives the operator's *already-authenticated* LLM CLIs (Claude Code, Codex) or any custom model — your data never has to leave the building.
- **Everything is explainable and provenance-tagged.** Every derived attribute, edge, score and judgment says where it came from and how confident it is.

## Install

```bash
git clone https://github.com/JoasASantos/CortexIntel && cd CortexIntel
./install.sh            # builds the release binary and scaffolds a demo
# or manually:
cargo build --release   # binary at ./target/release/cortex
```

## Quick start

```bash
cortex init --dir ./cortex-demo                                   # sample data
cortex run -i ./cortex-demo/reports.csv --domain fraud --offline  # deterministic, no cost
cortex serve --port 8787 --open                                   # the GUI in your browser
```

Outputs (`--out`, default `./cortex-out`): `case.json` (consolidated), `graph.json`, `report.md`, `audit.log.jsonl`.

## The intelligence engine

Every stage has a **deterministic core** + an optional **LLM agent**. Deterministic techniques built in:

| Layer | What it does |
|-------|--------------|
| **Potentiate** (enrich) | Normalizes + derives attributes (registrable domain of every URL, IP scope, activity hour) and materializes hub entities → richer correlation. |
| **Reference matching** | Matches file hashes against integrated feeds — **exact** and **perceptual near-duplicate** (Hamming) for altered/recompressed images (PhotoDNA / Project VIC style). |
| **Correlation + identity resolution** | Shared-hub linking (`same_ip_as`, `shares_domain_with`…) + probabilistic same-entity merge, reversible and explainable. |
| **Network science** | Betweenness (the **broker**), PageRank, communities (label propagation) + modularity. |
| **Anomaly detection** | Peer-relative outliers (robust median + MAD), precision-first. |
| **Risk + reward** | Transparent feature scorer + graph **risk propagation** + a **reward engine** that learns from analyst confirm/reject feedback. |
| **Link prediction** | Infers likely-but-absent edges (common neighbours + Adamic-Adar). |
| **Assessment** | Natural-language judgments (statement · confidence · evidence · action), per-vertical lens, pt/es/en. |

## Ready-made agents

45+ agents defined as **markdown files** (`agents/*.md`) — generic + niche-specific (finance/AML/KYC, cyber, child-protection, logistics, health, journalism, …). Classified by niche + category, **recommended by what your data actually contains**, with optional input forms and result-in-graph. Browse & run them in the GUI, or write your own in the editor. Scales to thousands by dropping `.md` files. → see [`docs/PLUGINS.md`](docs/PLUGINS.md).

## Interactive workspace (GUI)

- **Graph** — network / risk / community lenses, cluster collapse for scale, **draw connections manually with labels**, minimap, path-finder.
- **Map** — a flat world map that comes clear and fills geographically: **severity choropleth** per country, geolocated markers, connections, and **pluggable layers per project** (CCTV, air bases, units — any entity kind becomes a toggleable layer).
- **Intelligence** — decision panel (courses of action), planning timeline, competing hypotheses, decision matrix, next-best-action.
- **Agents** — recommended-for-this-data + full library + in-app editor.

## Integrations

Databases and clouds via the operator's own clients (`psql`, `mysql`, `bq`, `aws`, `gsutil`, `sqlcmd`, `mongoexport`) and HTTP APIs via `curl` — including **custom webhook/REST integrations with method, headers, body, and JWT / bearer / basic / API-key auth**. Portable **`.cortex`** manifests preset a whole run.

## Calibration

Thresholds are data-dependent. `CORTEX_CALIBRATE=1 cortex run …` measures the real distribution on your volume and recommends values; apply them via env vars (no rebuild). Full reference: [`docs/USAGE.md`](docs/USAGE.md).

## Docs

- [`docs/USAGE.md`](docs/USAGE.md) — commands, verticals, providers, calibration, env vars, integrations, `.cortex`.
- [`docs/PLUGINS.md`](docs/PLUGINS.md) — write agents, plugins (`manifest.json`) and transforms.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — what's next (incl. SIGINT / OSINT / HUMINT / GEOINT).

## Guardrails

The AI **supports** decisions — it never decides guilt/liability or takes irreversible action; it separates suspicion vs. evidence vs. inference; it references sensitive material by hash/id (never raw); it states confidence and flags what needs human review. Reference matches (incl. victim-ID) are **decision-support requiring human confirmation**.

## License & use

Built for authorized intelligence, investigative and decision-support work. Handle personal data under the applicable law (LGPD / GDPR / local). This is decision-support, never a definitive ruling.


## Cognitive layer (v0.0.2)

Every LLM call goes through a scored router plus memory/governance layers:

* **Model catalog & base score** (`src/llm/models.rs`) — Claude Opus 5 / Sonnet 5, Codex GPT-6 Astras / GPT-5.6 Sol · Luna · Terra / GPT-5.5, Gemini 2.5, Kimi, Qwen, DeepSeek (OpenAI-compatible APIs from `.env`), hermes / opencode / any CLI. Each model scores reasoning × speed × cost; the task tier (simple / standard / complex), JSON need, payload size and budget pressure pick the winner and the cross-vendor fallback order. Inspect in *Ajustes → Provedores & Roteamento* or `GET /api/models`.
* **Token / cost governor** (`src/llm/governor.rs`) — per-job budgets scaled to the workload, pressure feedback into routing, hard veto.
* **Prompt + semantic cache** (`src/llm/cache.rs`) — exact hash cache and near-duplicate (embeddings or lexical) cache, TTL-bound.
* **Prep / distill** (`src/llm/prep.rs`) — observation compressor (dedupe, truncate, head/tail windowing), language directive (pt-BR by default), reply distillation so only the essential JSON reaches the engine.
* **Memory & retrieval** (`src/memory.rs`) — project facts, grep + BM25 + vector hybrid retrieval, GraphRAG neighbourhood packing, context compression. `GET/POST /api/memory/*`.
* **Orchestrator** (`src/orchestrator.rs`) — dynamic tool registry (`GET /api/tools`), planner → worker → answer → critic loop for every question.
* **Event bus** (`src/bus.rs`) — every step is logged per job (`GET /api/jobs/status` returns `log[]`, shown live in the GUI console) and globally (`GET /api/events`).

See `.env.example` for all knobs.

## Scenario packs

* `scenarios/human-trafficking/` — **Operação Rota Silenciosa**: synthetic counter-trafficking case (recruitment → transport → exploitation → proceeds) with 3 datasets, a classifier plugin, 9 transforms (`trafficking` category) and 6 agents. See its README.

## Configurable API transforms (v0.0.3)

A transform with `runtime: "api"` carries a **declarative JSON spec** (no code) so operators wire any REST service from the GUI (Ajustes → Loja de Transforms) or a plugin manifest. Built-ins:

| category | transform | service |
|---|---|---|
| signals | Shodan host/search, Censys, GreyNoise, AbuseIPDB, LeakCheck, crypto-abuse, generic GET builder | key per service |
| phone | HLR/owner lookup, phone→linked accounts (WhatsApp/Telegram) | key + endpoint |
| face | per-provider face search — FaceCheck.ID, PimEyes, Search4Faces, FaceSearch/FaceOnLive, Lenso.ai, Betaface — plus generic, face compare, reverse image | key per provider (+ endpoint for some), image upload |
| geoint | geocode (OSM, no key), cameras nearby (OSM surveillance), places nearby (hotels/ATMs), Wi-Fi→location (WiGLE) | mostly no key |

**Spec** (`entrypoint`): `steps[]` (method, url, headers, query, json/form/body, save vars) + `map` (items path, kind, label, attributes, relation, lat/lon, confidence) + optional `extra[]`. Placeholders: `{label} {key} {attr.NAME} {param.NAME} {var.NAME} {lat} {lon} {file_b64} {label_digits} {label_enc}`; item fields as `item.path` or `{item.path}`. Params are declared in `params[]` and the GUI renders a form (text/number/file/select), including file upload for face search. Keys live in Ajustes → Chaves de API; per-run endpoints/files come from the form.

## Seed investigation (AI)

**Investigar com IA** (add-entity modal, entities panel, ⌘K): type a subject (name, phone, CPF, wallet…) + free-form context; the model derives the connected entities (aliases, phones, CPF/RG, e-mails, addresses, crypto wallets, URLs, social accounts, orgs, vehicles) and their relationships as **hypotheses** to confirm. A deterministic extractor guarantees real identifiers in the text (e-mail, CPF/CNPJ, BTC/ETH wallet, @handle, URL/domain) even offline. Endpoint: `POST /api/jobs {kind:"investigate"}` → merged into the graph via the existing proposal flow. Then run the API transforms to confirm each lead.

## Ontology (v0.0.3)

17 new entity kinds: vehicle, email, username, address, document, hash, credential, breach, celltower, wifi, certificate, camera, event, weapon, face, bankaccount (plus richer intra-record links and correlation hubs: same_email_as, same_handle_as, same_address_as, same_vehicle_as, same_bank_account_as, same_cell_as, same_wifi_as, same_file_as). Sensitive kinds (document, credential, face, victim, media…) are fingerprint-labelled and gated.


### Face-search providers & fan-out

Dedicated transforms: `face.facecheck` (FaceCheck.ID, 2-step upload+search), `face.pimeyes`, `face.search4faces` (VK/OK/TikTok/IG datasets), `face.facesearch` (FaceSearch/FaceOnLive), `face.lenso`, `face.betaface` (attributes + match). Each takes the image from a media/face/person entity (`attributes.path`) or a `file` param, keys from Ajustes → Chaves de API, per-run endpoints remembered per provider.

Add an entity + image and search in one shot: the **Adicionar entidade** modal (and the entities panel quick-add for media/face) opens the native file picker directly, with a "Busca facial em todos os provedores" checkbox that fans out across every installed+configured face provider on add. Results are merged as URLs/accounts tagged by provider and match band (`match:strong ≥90` / `match:likely 75–90` / `match:possible <75`). Also on any media/face/person node: right-click → "Buscar rosto (todos os provedores)", or ⌘K → "Busca facial".
