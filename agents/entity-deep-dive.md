---
name: Entity Deep Dive
description: Expand and profile a single entity and its immediate network.
domains: [*]
category: Investigation
tags: [expand, profile]
inputs: [entity:text:Entity label or id]
view: network
reflects: focus
---
Profile the entity {{entity}}: its attributes, its direct connections, its role in the network, and what is known vs. inferred vs. missing. Recommend the next step to confirm its role.
