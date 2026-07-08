---
name: C2 & Beaconing Detector
description: Flag likely command-and-control / beaconing patterns.
domains: [cybersecurity]
category: Threat Intel
tags: [c2, beaconing]
triggers: [ip, domain, device]
reflects: focus
---
Identify likely command-and-control: hosts talking to the same external infrastructure on a regular cadence, or fanning out from one internal device. Flag the suspected C2 nodes and the beaconing clients.
