---
id: 6
title: 'Substantive agent prompts: Red Flags + anti-rationalization framing'
type: doc
state: doc
priority: p2
blocked_by:
- 7
- 10
epic: agent-prompt-content
created: 2026-05-10T02:03:51Z
created_by: maintainer
updated: 2026-05-11T04:04:00Z
---
The Claude Code adapter (slice 18, src/adapter/claude_code/agents.rs) renders structured-but-spare prompts from agent-roster.md content: trigger / inputs / outputs / exit conditions / scope fences / pointers. Per CLAUDE.md "Skills are behavior-shaping code, not prose. Changes to Red Flags tables, rationalization lists, and 'EXTREMELY-IMPORTANT' framing need eval evidence."

Current prompts are functionally sufficient (an agent reading them will know what to do and what not to do) but lack the load-bearing behavioral framing carried forward from v0.1: the "Red Flags" tables that anti-rationalize specific failure modes, the EXTREMELY-IMPORTANT blocks that lock in procedural gates.

Scope:
- For each of the 10 agents, review the v0.1 inherited prompt content (since deleted in slice 23 — recoverable from git history at b1bbbe0~1) and identify which Red Flags / anti-rationalization framing transfers to the v2 contract.
- Author the per-agent additions in src/adapter/claude_code/agents.rs's AgentDef.* fields. Don't synthesize prose without eval evidence; pull verified material from v0.1 and revise for v2 vocabulary (e.g., "agent:plan" → "state: plan").
- Eval (per #7) gates each prompt rev.

Blocked on #7 (eval framework) before substantive content lands.
