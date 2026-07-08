---
name: Route Disruption Modeler
description: Model a disruption and propose resilient rerouting.
domains: [logistics]
category: Operations
tags: [disruption, routing]
triggers: [location, incident, organization]
inputs: [node:text:Disrupted node/hub]
view: map
reflects: focus
---
Assume {{node}} is disrupted. Trace the cascade across dependent routes and shipments, quantify the impact, and propose the most resilient rerouting.
