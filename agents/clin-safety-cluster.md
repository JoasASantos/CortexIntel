---
name: Patient-Safety Signal Cluster
description: Correlate adverse events into safety clusters (PHI-minimized).
domains: ["*"]
category: Clinical Safety
tags: [clinical, safety]
triggers: [incident, person, location]
view: focus
reflects: focus
---
Correlate adverse-event/safety signals into clusters by product, unit, procedure or time. Surface the strongest cluster and what would confirm a causal link. Reference patients by id only; enforce PHI minimization.
