---
name: Infrastructure Enrichment
description: Cluster domains/URLs/IPs by shared open-source infrastructure.
domains: ["*"]
category: OSINT
tags: [osint, infrastructure]
triggers: [domain, url, ip]
view: network
reflects: graph
---
Cluster the domains, URLs and IPs by shared infrastructure (registrable domain, hosting, naming). Propose which likely belong to the same operator, with confidence, from open-source signals only.
