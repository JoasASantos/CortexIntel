---
name: Selector Activity & Bursts
description: Detect bursts and unusual activity windows in signals.
domains: ["*"]
category: SIGINT
tags: [sigint, temporal]
triggers: [communication, account, device]
view: timeline
reflects: focus
---
Detect activity bursts and unusual windows in the signals metadata (spikes in volume, off-hours activity, sudden new selectors). Flag the strongest anomalies and the accounts/devices involved.
