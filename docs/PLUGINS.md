# CortexIntel — Extending: Agents, Plugins & Transforms

Three ways to extend CortexIntel, from easiest to most powerful:

1. **Agents** — markdown prompts that work over the current graph (no code).
2. **Plugins** — a `manifest.json` that teaches the engine new fields, risk
   signals and prompt context for a vertical (no code).
3. **Transforms** — a small script that takes one entity and returns new
   entities/relationships (Maltego-style enrichment).

---

## 1. Agents (`agents/*.md`)

An agent is a markdown file with YAML-ish frontmatter + a prompt body. Drop it in
`$CORTEX_AGENTS_DIR` (default `<data>/agents/`) — it appears in the GUI's Agents
browser immediately. This scales to thousands of agents.

```markdown
---
name: AML Layering Detector
description: Detect layering/structuring across transactions in a time window.
domains: [fraud, finance]        # ["*"] = generic; a list = niche-scoped
category: AML                    # finer grouping within the niche
tags: [aml, laundering]
triggers: [payment, wallet, account]   # entity kinds → drives recommendation
inputs: [window_days:number:Look-back (days), threshold:number:Structuring threshold]
view: timeline                   # network | map | timeline (switch after running)
auto: false                      # run automatically on ingest?
reflects: graph                  # answer | graph | focus
---
Look for layering: many small transfers within {{window_days}} days, amounts just
under {{threshold}}. Flag the chains and the accounts that anchor them.
```

- **`{{field}}`** placeholders are filled from `inputs` via a form the GUI opens
  before running.
- **`reflects`**: `answer` (text only), `graph` (merge proposed entities/edges),
  `focus` (isolate/highlight).
- **Recommendation**: agents whose `triggers` match the entity kinds present in
  the data are surfaced under "Recommended for this data", ranked by fit.

Create/edit agents in the GUI (Agents → New / ✎) or via
`POST /api/agents/save { id, content }`.

---

## 2. Plugins (`manifest.json`)

A plugin extends the deterministic engine for a vertical without code. Install via
the GUI (Settings → Plugins) or `POST /api/plugins/install`; stored in
`<data>/plugins/*.json` and loaded at startup.

```json
{
  "name": "Telco Fraud Pack",
  "version": "1.0.0",
  "author": "you",
  "description": "Field mappings + risk signals for telecom fraud.",
  "domains": ["telecom"],
  "field_mappings": [
    { "field": "imsi", "kind": "device" },
    { "field": "msisdn", "kind": "account" },
    { "field": "cell_id", "kind": "location" }
  ],
  "risk_signals": [
    { "token": "sim_swap", "weight": 0.8 },
    { "token": "bypass", "weight": 0.7 }
  ],
  "prompt_addon": "Emphasize SIM-box and interconnect-bypass patterns.",
  "enabled": true
}
```

- **`field_mappings`** — teach extraction that a column maps to an entity kind.
- **`risk_signals`** — tokens (matched in tags/attributes/labels) → risk weight.
- **`prompt_addon`** — appended to agent system prompts for this vertical.
- **`domains`** — empty = applies to all verticals.

---

## 3. Transforms

A transform enriches ONE entity into new entities/relationships (query a source,
expand infrastructure, resolve an identity). Install a manifest via the GUI
(Graph → right-click → Transforms) or `POST /api/transforms/install_manifest`.

**Manifest:**
```json
{
  "id": "whois-expand",
  "name": "WHOIS Expand",
  "description": "Resolve a domain's registrant + name servers.",
  "input_kinds": ["domain"],
  "runtime": "python",
  "command": "python3 /path/whois_expand.py",
  "requires_api_key": false
}
```

**I/O contract** — the command reads JSON on **stdin** and writes JSON on
**stdout**:

```jsonc
// stdin
{ "input": { "kind": "domain", "label": "example.com", "attributes": {} },
  "params": {}, "api_key": "" }

// stdout
{ "entities": [ { "kind": "person", "label": "Registrant X", "attributes": {} } ],
  "relationships": [ { "source": "example.com", "type": "registered_by",
                       "target": "Registrant X", "confidence": 0.8 } ] }
```

Returned entities/relationships merge into the graph (marked as derived). Keep
transforms deterministic where possible; anything requiring a key declares
`requires_api_key: true` and receives it via the `api_key` field (never logged).

---

## Where things live

```
<data>/                 # $CORTEX_HOME_DIR or ~/.cortexintel
  agents/*.md           # agent library
  plugins/*.json        # installed plugins
  transforms/           # installed transforms
  reward.json           # reward/feedback store
  projects/             # saved projects
refs/                   # (your path) reference-source feeds → CORTEX_REFS
```
