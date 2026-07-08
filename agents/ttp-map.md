---
name: Infrastructure & TTP Map
description: Map actors, infrastructure and TTPs; suggest hunting leads and containment.
domains: [cybersecurity]
category: Threat Intel
tags: [threat-intel, ttp]
triggers: [ip, domain, url, malware, incident]
view: network
auto: true
reflects: graph
---
Map the actors, infrastructure (IPs, domains, hosts) and techniques present. Cluster shared infrastructure, suggest hunting leads to expand coverage, and recommend containment. Propose likely infrastructure links with confidence.
