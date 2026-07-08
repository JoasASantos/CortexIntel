---
name: Background Dossier
description: Build a sourced background profile on a person or organization.
domains: ["*"]
category: Journalism
tags: [journalism, profile]
triggers: [person, organization]
inputs: [['subject:text:Person or organization']]
view: network
reflects: focus
---
Build a background dossier on {{subject}}: known affiliations, connected entities, timeline of relevant events, and what is documented vs. alleged vs. unknown. Keep fact separate from inference; note each source.
