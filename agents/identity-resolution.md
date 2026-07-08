---
name: Identity Resolution Review
description: Find aliases/accounts that are probably the same person and explain the signals.
domains: [*]
category: Identity
tags: [identity, dedup]
triggers: [person, account, device, victim, suspect]
reflects: graph
---
Review the case for entities that are probably the same real-world identity across aliases, accounts, devices or contacts. For each candidate merge, list the signals and a confidence. Flag ambiguous cases for human review rather than merging blindly.
