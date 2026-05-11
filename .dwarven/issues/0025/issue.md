---
id: 25
title: 'evals/MANUAL-CHECKLIST.md: bait-prompt eval pass per agent'
type: doc
state: doc
priority: p1
epic: agent-validation
created: 2026-05-11T18:49:17Z
created_by: maintainer
updated: 2026-05-11T18:50:05Z
---
Companion to the YAML-driven eval framework. Manual checklist of one bait prompt per agent for validating the substantive prompt framing (#6) directly in a Claude Code session without an API key.

Each agent gets one prompt designed to tempt the load-bearing red flag, with explicit pass/fail criteria. The maintainer runs the prompt by dispatching the agent (\`/spec\`, \`/test\`, etc.) in a Claude Code session in this repo and observes whether the framing holds.

Location: \`evals/MANUAL-CHECKLIST.md\` — alongside the YAML scenarios so the eval directory holds both forms of validation.

Also: append a 'Reading results' section explaining the three failure modes (full bait, partial bait, refusal-for-wrong-reason) and how to adjust the registry.rs framing in response.
