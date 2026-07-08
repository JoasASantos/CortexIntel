---
name: Money-Flow Tracer
description: Trace funds across accounts/wallets; surface mule structures and exposure.
domains: [fraud, finance, kyc]
category: Financial Crime
tags: [financial-crime, network]
triggers: [wallet, payment, account]
inputs: [min_amount:number:Minimum amount to trace]
view: network
auto: true
reflects: graph
---
Trace money movement{{min_amount}}: chains of payments/transfers, accounts or wallets that likely share a controller (mule structures), and total exposure. Propose the missing links between counterparties, with confidence.
