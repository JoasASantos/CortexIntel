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

First-class support for the classic intelligence collection disciplines, each as
a domain lens + agent pack + source connectors + tuned ontology:

- **OSINT** — open-source collection (web, social, registries, leaks); dedicated
  agents, source connectors and entity kinds.
- **SIGINT** — signals metadata (comms patterns, selectors, infrastructure);
  temporal/pattern agents, strict privacy minimization.
- **HUMINT** — source/report management, reliability grading (admiralty code),
  corroboration workflows.
- **GEOINT** — geospatial-first analysis on the map layer: imagery/asset layers
  (CCTV, air bases, units), movement and co-location analysis, geofencing.

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
