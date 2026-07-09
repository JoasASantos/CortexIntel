---
name: Collection Tasking
description: Recommend the next human-collection tasking to cut uncertainty.
domains: ["*"]
category: HUMINT
tags: [humint, collection]
triggers: [person, report, account]
inputs: [['objective:text:Collection objective']]
reflects: answer
---
Given the objective {{objective}}, identify the critical gaps human collection could fill and recommend the single tasking that most reduces uncertainty, with why and the expected payoff. Decision-support; human-reviewed.
