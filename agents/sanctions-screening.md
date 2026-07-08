---
name: Watchlist / Sanctions Screening
description: Screen entities against a provided watchlist.
domains: [fraud, finance, kyc, government]
category: Compliance
tags: [sanctions, watchlist]
triggers: [person, organization, account]
inputs: [list_name:text:Watchlist name or source]
reflects: answer
---
Screen the case entities against the watchlist {{list_name}}. Report likely matches (exact and fuzzy) with the matching signals and a confidence. Every hit is decision-support requiring human confirmation.
