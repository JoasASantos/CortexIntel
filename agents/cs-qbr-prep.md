---
name: QBR / Renewal Prep
description: Prepare a QBR / renewal brief for an account.
domains: ["*"]
category: Customer Success
tags: [cs, renewal, qbr]
triggers: [account, person, payment]
inputs: [['account_name:text:Account']]
reflects: answer
---
Prepare a QBR/renewal brief for {{account_name}}: value delivered, adoption trend, open risks, stakeholders and their sentiment, renewal risk, and the 3 talking points that most move the renewal. Decision-ready for the CSM.
