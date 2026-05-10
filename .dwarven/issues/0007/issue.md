---
id: 7
title: Eval framework for agent prompts
type: arch
state: architect
priority: p2
blocks:
- 6
epic: agent-prompt-content
created: 2026-05-10T02:04:01Z
created_by: maintainer
updated: 2026-05-10T02:04:06Z
---
Per CLAUDE.md the Red Flags tables and anti-rationalization framing in agent prompts "need eval evidence" before they land. Today there is no eval framework — substantive prompt changes would either ship blind or be tested informally.

Scope:
- Define what "eval evidence" means for an agent prompt revision: a fixed scenario set per agent (10 × N scenarios), expected behavior, automated grading criteria.
- Decide on infrastructure: do we record agent runs and replay? Do we use Claude API directly with a mock dwarven CLI? Do we use a test repo with the real adapter and real Claude Code?
- Scope the smallest meaningful eval: maybe 3 critical-failure scenarios per agent (the "should-have-refused" cases) is enough to catch regressions without becoming a research project.
- Output: docs/architecture/agent-eval.md spec'ing the framework; a reference scenario file; a runner script.

Blocks #6 (substantive prompt content). Maintainer time/judgment-heavy; this is not a "delegate-and-walk-away" item.
