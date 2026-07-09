---
name: Compliance Gap Check
description: Check the case against a stated compliance requirement.
domains: ["*"]
category: Legal / Compliance
tags: [legal, compliance]
triggers: [report, organization, person]
inputs: [['requirement:text:Requirement or policy']]
reflects: answer
---
Check the case against {{requirement}}: where is the evidence of compliance, where are the gaps, and what would close each gap. Decision-support for a compliance reviewer.
