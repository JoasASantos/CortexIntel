---
name: Selector / Handle Correlation
description: Find the same username/handle reused across platforms.
domains: ["*"]
category: OSINT
tags: [osint, selector]
triggers: [account, person]
view: network
reflects: graph
---
Correlate selectors: the same username/handle appearing across platforms is a strong same-actor signal. List the candidate identities, the shared selectors, and a confidence. Flag ambiguous ones for review.
