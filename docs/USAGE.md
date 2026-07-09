# CortexIntel — Usage

Complete reference for running CortexIntel: commands, verticals, providers,
integrations, calibration, and every environment variable.

## Commands

```
cortex run     -i <src…> --domain <v> [--data-type <t>] [--provider <p>] [--offline] [-o <dir>]
cortex serve   [--port 8787] [--open]      # local GUI in the browser
cortex desktop [--port 8787]               # native app if built, else browser
cortex menu                                # interactive: pick vertical, type, provider, sources
cortex agents  --domain <v>                # list the pipeline agents for a vertical
cortex doctor                              # check Claude / Codex / custom model availability
cortex sources <path>                      # inspect a CSV/JSON source or .mcp manifest
cortex init    --dir <dir>                 # scaffold demo data
```

`cortex run` also accepts a **`.cortex` manifest**: `cortex run -i case.cortex`
expands `{ "name", "domain", "inputs": [...], "out" }` into a full run (paths are
resolved relative to the manifest).

## Verticals (`--domain`)

`child-protection`, `cybersecurity`, `fraud`, `finance`, `kyc`, `health`,
`commerce`, `logistics`, `military`, `government`, `legal`, `insurance`,
`telecom`, `energy`, `manufacturing`, `real-estate`, `education`, `nonprofit`,
`generic`. The vertical changes the vocabulary, agents and what counts as risk —
the engine is the same.

## Providers (`--provider`)

- `auto` (default) — Claude → Codex → custom, complexity-routed, with fallback.
- `claude` — Claude Code CLI (subscription, headless).
- `codex` — ChatGPT Codex CLI.
- `custom` — any CLI model via `CORTEX_LLM_CMD` (Ollama, `llm`, local, wrappers).
- `mock` / `--offline` — deterministic stub, no external calls, no cost.

`cortex doctor` reports which backends are installed and authenticated.

## Data sources

- **CSV / TSV**, **JSON / JSONL / NDJSON** — read directly.
- **Databases / clouds** — via the operator's own clients: `psql` (postgres),
  `mysql`, `bq` (BigQuery), `sqlcmd` (SQL Server), `mongoexport` (MongoDB),
  `aws`/`gsutil` (S3/GCS data lakes). Secrets go via env vars, never the CLI line
  or the saved config.
- **HTTP / REST / webhook / Elasticsearch** — via `curl`. Connector config keys:
  `endpoint` (required), `method`, `headers` (object), `body` (string|json), and
  auth: `jwt` | `token` (+ `user` for basic) | `api_key` (+ `api_key_header`).
  Use `records_path` in a source/`.mcp` config to point at a nested array (e.g.
  Elasticsearch `hits.hits`).
- **MCP** — an `.mcp` manifest emits a fetch-plan you run through an MCP-enabled
  agent, then re-ingest.

## Reference sources (known-hash matching)

Point `CORTEX_REFS` at a JSON file or directory of feeds:

```json
{ "source": "Known set", "category": "known_csam_reference|malware|watchlist",
  "severity": "critical|high|medium", "kind": "hash|perceptual",
  "values": ["<sha256>", "<md5>", "<phash-hex>"] }
```

`kind: hash` = exact byte-identical match; `kind: perceptual` = near-duplicate
by Hamming distance (catches altered/recompressed images). Matches raise risk and
surface an assessment requiring **human confirmation**.

## Calibration

Thresholds depend on your data volume/shape. Measure, then apply:

```bash
# 1) measure + recommend on your data
CORTEX_CALIBRATE=1 CORTEX_REFS=/refs cortex run -i /your/data --domain <v> --offline -o /tmp/cal
# 2) apply the recommended cuts (no rebuild)
CORTEX_ANOMALY_Z=3.0 CORTEX_LINK_MIN_SHARED=2 CORTEX_PHASH_MAXDIST=8 cortex run -i /your/data --domain <v> --offline
```

## Reward / feedback

The reward engine learns from analyst verdicts. Submit feedback (GUI or API):
`POST /api/feedback { "key": "tag:known-file-hash", "signal": 1, "weight": 3 }`.
Keys are dimensions (`kind:*`, `tag:*`, `action:*`); a bounded ±0.15 adjustment
then nudges future risk scoring (shown as a `feedback:` factor). Stored in
`<data>/reward.json`.

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `CORTEX_HOME_DIR` | `~/.cortexintel` | Data dir (accounts, projects, agents, reward, refs). |
| `CORTEX_AGENTS_DIR` | `<data>/agents` | Where the markdown agent library lives. |
| `CORTEX_REFS` | — | Reference-source JSON file or directory. |
| `CORTEX_CALIBRATE` | — | `1` → print a threshold-calibration report. |
| `CORTEX_ANOMALY_Z` | `3.5` | Robust-z threshold for anomaly flags. |
| `CORTEX_LINK_MIN_SHARED` | `2` | Min shared neighbours to predict a link. |
| `CORTEX_LINK_TOPK` | `12` | Max predicted links per run. |
| `CORTEX_PHASH_MAXDIST` | `10` | Max Hamming distance for a perceptual match. |
| `CORTEX_NO_ENRICH` | — | `1` → skip the potentiate/enrich stage. |
| `CORTEX_LLM_CMD` | — | Command for the `custom` model provider. |
| `CORTEX_LLM_RETRIES` | `1` | Extra retries per LLM attempt (backoff). |
| `CORTEX_MODEL_COMPLEX/STANDARD/SIMPLE` | claude-opus-4-8 / claude-sonnet-5 / gpt-5.5 | Model per complexity tier in `auto`. |
| `CORTEX_CLAUDE_BIN` / `CORTEX_CODEX_BIN` | claude / codex | Override the CLI binary. |
| `CORTEX_CLAUDE_USER` | auto (SUDO_USER → console owner) | When running as root, run Claude Code as this normal user (uses their subscription). |
| `CORTEX_CLAUDE_NO_DROP` | — | `1` → don't drop to a user; run Claude directly (with the IS_SANDBOX escape). |

## Notes

- Claude Code refuses `--dangerously-skip-permissions` as **root/sudo** — run as a
  normal user, or use `--provider codex` / `custom` / `--offline`.
- No local database: the knowledge graph is built in memory and written as JSON.
