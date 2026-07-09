---
name: Account Plan / Whitespace
description: Build an account plan and find whitespace.
domains: ["*"]
category: Commercial
tags: [sales, account-plan]
triggers: [account, person, organization]
inputs: [['account_name:text:Account']]
view: network
reflects: answer
---
Build an account plan for {{account_name}}: current footprint, key relationships, whitespace (products/teams not yet sold), risks, and the sequenced plays to grow the account. Identify the best entry champion for each whitespace.
