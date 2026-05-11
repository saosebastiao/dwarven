---
id: 10
title: Implement eval runner per docs/architecture/agent-eval.md
type: feature
state: done
priority: p2
blocks:
- 6
epic: agent-prompt-content
created: 2026-05-11T04:03:50Z
created_by: architect
updated: 2026-05-11T04:27:57Z
---
Spec: `docs/architecture/agent-eval.md` (issue #7).

Implements `examples/eval_runner.rs` and the supporting harness:

- Anthropic SDK integration (add `anthropic-sdk` or hand-rolled HTTP client; the SDK ecosystem is unstable so hand-rolled with `reqwest` + serde may be safer).
- YAML scenario loader (`serde_yaml` already in tree).
- Materialized-prompt loader that reads from `crate::adapter::claude_code::agents::roster()` and `crate::adapter::claude_code::agents::render()`. The eval runner consumes the same prompt the production adapter ships — single source of truth per the architecture doc.
- Mock dwarven CLI for the tools/Bash patterns the agent invokes — returns canned issue JSON from the scenario's `fixture_issues` for read commands; records but does not execute mutating commands.
- Mock Edit/Write that records calls and returns success stubs.
- Real Read against the actual repository (so the agent can consult specs/source it would genuinely read).
- AskUserQuestion answer queue from `follow_up_answers`.
- Assertion runner per the spec's pattern-matching shape (tool name + path_glob + contains).
- Optional response_judge: second Claude call evaluating the rubric; parse first token as PASS|FAIL.
- CLI flags: `--agent <name>`, `--scenario <path>`, default = run all.
- Refuse to start without `ANTHROPIC_API_KEY`.
- Output format per the spec doc (one line per scenario, aggregate summary at end, failures show the offending assertion).

Tests: the runner itself should have unit tests for the assertion-pattern matcher and the scenario YAML parser; the runner's end-to-end behavior is exercised by running the two reference scenarios under #7 (when ANTHROPIC_API_KEY is set; otherwise skipped).

Scope fences: this is implementation work, type:feature, default state:pm.
