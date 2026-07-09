---
name: Geofence Watch
description: Flag entities inside a region of interest.
domains: ["*"]
category: GEOINT
tags: [geoint, geofence]
triggers: [location, device, person]
inputs: [['region:text:Region of interest (name or bbox)']]
view: map
reflects: focus
---
Flag which entities fall inside the region of interest {{region}} and summarize what is happening there — who/what is present, the risk level, and how it connects to the rest of the case.
