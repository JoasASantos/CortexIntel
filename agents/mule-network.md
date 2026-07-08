---
name: Mule Network Detector
description: Detect rings of mule accounts moving funds on behalf of a controller.
domains: [fraud, finance]
category: Financial Crime
tags: [mule, network]
triggers: [account, payment, device]
view: network
reflects: graph
---
Detect mule rings: accounts that receive and quickly forward funds, share devices/IPs, or move in coordinated bursts. Cluster them and identify the likely controller. Propose the intra-ring links with confidence.
