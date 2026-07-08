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
- `DATA.md` / `DESIGN.md` — data model & UX design.

## Guardrails

The AI **supports** decisions — it never decides guilt/liability or takes irreversible action; it separates suspicion vs. evidence vs. inference; it references sensitive material by hash/id (never raw); it states confidence and flags what needs human review. Reference matches (incl. victim-ID) are **decision-support requiring human confirmation**.

## License & use

Built for authorized intelligence, investigative and decision-support work. Handle personal data under the applicable law (LGPD / GDPR / local). This is decision-support, never a definitive ruling.
