---
name: OSINT Footprint Mapping
description: Map an entity's open-source footprint across platforms and sources.
domains: ["*"]
category: OSINT
tags: [osint, footprint]
triggers: [account, person, domain, url]
inputs: [['subject:text:Subject entity']]
view: network
reflects: graph
---
Map the open-source footprint of {{subject}}: connected accounts, handles, domains and mentions across sources. Propose which belong to the same actor (selector reuse) with confidence. Open-source only; no intrusion.
