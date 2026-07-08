---
name: Procurement Collusion
description: Detect bid-rigging / collusion patterns in procurement.
domains: [government, legal]
category: Integrity
tags: [procurement, collusion]
triggers: [organization, person, payment]
view: network
reflects: graph
---
Detect likely bid-rigging or collusion: bidders sharing owners/addresses/timing, rotating winners, or coordinated pricing. Flag the suspected ring and the pattern.
