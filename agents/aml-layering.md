---
name: AML Layering Detector
description: Detect layering/structuring across transactions in a time window.
domains: [fraud, finance]
category: AML
tags: [aml, laundering, temporal]
triggers: [payment, wallet, account]
inputs: [window_days:number:Look-back window (days), threshold:number:Structuring threshold]
view: timeline
reflects: graph
---
Look for layering and structuring: many small transfers splitting a larger sum, rapid movement across accounts, or amounts just under {{threshold}} within the last {{window_days}} days. Flag the chains and the anchoring accounts.
