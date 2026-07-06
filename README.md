# CortexIntel

**Agnostic data-collection & intelligence engine with an LLM decision layer.**

CortexIntel is a Rust CLI (`cortex`) that turns heterogeneous data feeds into a
correlated, prioritized, auditable intelligence picture that supports human
decision-making. It is **domain-agnostic**: the same engine serves
child-protection / victim-identification, cybersecurity / threat-intel, fraud &
AML, healthcare safety, commerce and logistics — you pick the vertical.

The "brains" are not embedded model weights. CortexIntel drives the operator's
**already-authenticated LLM CLIs**:

- **Claude Code** (subscription) — headless print mode with `--dangerously-skip-permissions`.
- **ChatGPT Codex** — `codex exec`.

Each pipeline stage is run by a specialized **agent** (persona + prompt + JSON
contract) routed through these backends. Every stage also has a **deterministic
heuristic core**, so the full pipeline produces a graph, risk scores and audit
trail even offline (`--offline`) with no external calls and no cost.

## The pipeline

Mirrors the operator flow from the brief:

```
Secure ingestion
   → Normalization & deduplication
   → Entities (person, account, device, IP, URL, file, case, location, …)
   → Graph correlation
   → Risk prioritization
   → Investigation, victim protection & evidence
   → Audit, retention & legal disposal
```

| Stage | Deterministic core | LLM agent |
|-------|--------------------|-----------|
| Ingestion | CSV / JSON / NDJSON readers, MCP fetch-plan | — |
| Classification | declared type / heuristics | `*.classifier` |
| Extraction & dedup | field-map + indicator scanners (IP/URL/email/hash/wallet/domain) | `*.extractor` |
| Correlation | shared-hub linking (`same_ip_as`, `same_device_as`, …) | `*.correlator` |
| Risk | transparent feature scorer (DATA.md risk features) | `*.risk` |
| Investigation | prioritized entity/relationship brief input | `*.investigator` |
| Audit | append-only JSONL log + retention/disposal policy | `*.auditor` |

No local database is required. Data is read in, a knowledge graph is built in
memory, and results are written as JSON + Markdown.

## Install

```bash
cargo build --release
# binary: ./target/release/cortex
```

## Quick start

```bash
# 1) scaffold a sample dataset + MCP manifest
cortex init --dir ./cortex-demo

# 2) run fully offline (deterministic, no LLM, no cost)
cortex run -i ./cortex-demo/reports.csv --domain child-protection --offline

# 3) run with a real LLM backend
cortex run -i ./cortex-demo/reports.csv --domain fraud --provider auto
```

Outputs (in `--out`, default `./cortex-out`):

- `case.json` — the consolidated document (DATA.md "estrutura final do dado consolidado").
- `graph.json` — `{nodes, edges}` ready for the DESIGN.md graph workspace.
- `report.md` — human-readable investigative brief.
- `audit.log.jsonl` — append-only audit trail.

## Commands

```
cortex run       -i <src…> --domain <v> [--data-type <t>] [--provider <p>] [--offline] [-o <dir>]
cortex serve     [--port 8787] [--open]  # local GUI: opens the workspace in your browser
cortex menu      # interactive: pick vertical, data type, provider, sources
cortex agents    --domain <v>            # list specialized agents
cortex doctor                            # check Claude/Codex availability
cortex sources   <path>                  # inspect a CSV/JSON source or MCP manifest
cortex init      --dir <dir>             # scaffold demo data
```

### GUI — two ways to run

The desktop workspace (see `DESIGN.md`) can run either way, both driving the same engine:

- **`cortex serve`** — a single self-contained binary that embeds the frontend and
  serves it over `http://127.0.0.1:8787`. Open that URL in a normal browser
  (Chrome/Safari/Firefox). No graphical session needed; native selects and
  everything work. This is the easiest way to run locally.
- **Tauri app** — a native macOS window: `cd gui/src-tauri && cargo tauri dev`.

The same `app.js` auto-detects its transport (Tauri IPC → local HTTP → offline
mock sample), so it works in the app, in the browser, and as a static file.

Verticals (`--domain`): `child-protection`, `cybersecurity`, `fraud`, `health`,
`commerce`, `logistics`, `generic`.

Providers (`--provider`): `auto` (Claude → Codex fallback), `claude`, `codex`,
`mock` (offline).

## Data sources & MCP

- **CSV / TSV**, **JSON / JSONL / NDJSON** — read directly, one record per row/object.
- **MCP** — an `.mcp` manifest (JSON) declares an MCP `server` + `tool` +
  `arguments`. Since MCP transports are executed by the agent runtime (Claude and
  Codex have live MCP access), CortexIntel emits a **fetch plan** you run through
  an MCP-enabled agent, then re-ingest the returned rows. Example manifest:

  ```json
  { "server": "claude_ai_Google_Drive", "tool": "search_files",
    "arguments": {"query": "case intake export"}, "records_path": "files" }
  ```

## AI guardrails (enforced in every agent prompt)

From DATA.md, generalized to any vertical:

- The AI **supports** decisions; it never decides guilt/liability or takes
  irreversible action.
- It separates suspicion vs. evidence vs. inference vs. confirmed decision.
- It does not surface sensitive content beyond operational need (sensitive
  entities are referenced by hash/id, never raw).
- It explains **why** it prioritized something, states confidence and limits,
  and flags anything needing human review.

## Notes

- Claude Code refuses `--dangerously-skip-permissions` when run as **root/sudo**.
  Run CortexIntel as a normal user, or use `--provider codex` / `--offline`.
  In `auto` mode the router automatically falls back to Codex.
- Model overrides: `--claude-model`, `--codex-model`. Binary overrides:
  `CORTEX_CLAUDE_BIN`, `CORTEX_CODEX_BIN`.
```
