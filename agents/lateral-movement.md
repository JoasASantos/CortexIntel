---
name: Lateral Movement Trace
description: Trace likely host-to-host movement across the environment.
domains: [cybersecurity]
category: DFIR
tags: [lateral-movement, dfir]
triggers: [device, account, ip, incident]
view: network
reflects: graph
---
Trace likely lateral movement: sequences of accounts/hosts that indicate an attacker pivoting through the environment. Propose the movement path and the pivot points to contain first.
