---
name: Pattern of Life
description: Surface an entity's recurring places and times.
domains: ["*"]
category: GEOINT
tags: [geoint, pattern-of-life]
triggers: [person, account, device, location]
inputs: [['subject:text:Subject entity']]
view: map
reflects: answer
---
For the subject {{subject}}, surface the pattern of life: recurring locations, the times/rhythm of presence, and the anchor sites (home/work/meeting). Separate the well-established pattern from one-off outliers. Privacy-minimized, human review required.
