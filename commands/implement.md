---
description: "Dispatch the Implementation agent to drive failing tests to GREEN (per R3.7)"
---

Use the `Agent` tool with `subagent_type="implementation"` to dispatch the Implementation agent.

Pass this prompt:

```
The maintainer invoked /implement with: $ARGUMENTS

Follow your defined process (orient → confirm RED → GREEN loop → REFACTOR → push → open PR → update issue → report). TDD discipline (RED-GREEN-REFACTOR) is mandatory. Per R3.7, you have no AskUserQuestion — escalate via agent:maintainer if blocked.
```

If `$ARGUMENTS` is empty, ask the maintainer "Which `agent:implement` issue would you like to work on?" and use their answer.

Do not edit any files yourself (R4.4).
