# CortexIntel — Roadmap

## v0.0.1 (current)

The full `data → information → intelligence` loop, deterministic and offline,
with an optional multi-LLM / custom-model layer:

- **Potentiate**: enrichment (derived attributes + hub entities), reference-source
  matching (exact + perceptual near-duplicate hash).
- **Correlate**: shared-hub correlation, probabilistic identity resolution.
- **Analyze**: network science (broker / PageRank / communities), anomaly
  detection, risk scoring + risk propagation + reward/feedback engine, link
  prediction.
- **Decide**: natural-language assessments, decision panel, next-best-action,
  planning timeline; threshold calibration.
- **Agents**: 45+ markdown agents (generic + niche), recommendation by data,
  in-GUI editor, result persistence.
- **Workspace**: graph (network/community lenses, manual labeled connections),
  flat choropleth world map with pluggable per-project layers, intelligence view.
- **Integrations**: DB/cloud via native clients; HTTP/REST/webhook/Elasticsearch
  with JWT / bearer / basic / API-key auth; `.cortex` project manifests.
- **Governance**: append-only audit, retention/disposal, human-review gates,
  hash-referenced sensitive material.

## v0.0.2 — Intelligence disciplines

First-class support for the classic intelligence collection disciplines, each a
deterministic signal in the pipeline + an agent pack:

- **GEOINT** ✅ — geospatial co-location correlation (haversine) feeding the map
  layer + pluggable per-project layers (CCTV, air bases, units); 6 agents.
- **HUMINT** ✅ — source/report reliability grading with the NATO Admiralty Code
  (source A–F × info 1–6) + corroboration; 3 agents.
- **SIGINT** ✅ — communication-pattern correlation (co-communication, metadata
  only, privacy-minimized); 3 agents.
- **OSINT** ✅ — selector/handle reuse across sources (same-actor candidates) +
  infrastructure clustering; 4 agents.

Next for the disciplines: dedicated source connectors, tuned per-discipline
ontology, and imagery/asset ingestion for GEOINT.

## Beyond

- **Continuous / streaming intelligence** — watchlists, standing queries,
  change detection between snapshots (living picture, not one-shot).
- **Neural layer** — graph embeddings + ANN index for similarity at scale, and
  facial embeddings for victim identification (with the governance block:
  chain-of-custody hash chain, RBAC, PII redaction, air-gapped operation).
- **Collaboration** — real-time multi-analyst, cross-case correlation.
- **In-GUI agent/plugin marketplace** — browse, install and share `.md`/manifest
  extensions.

Disciplines and neural capabilities that touch sensitive/biometric data ship with
mandatory human-in-the-loop confirmation and are gated behind the production
governance block.
