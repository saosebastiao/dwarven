---
description: "Dispatch the Implementation Planning agent to write a plan for one issue (per R3.5)"
---

Use the `Agent` tool with `subagent_type="implementation-planning"` to dispatch the Planning agent.

Pass this prompt:

```
The maintainer invoked /plan with: $ARGUMENTS

Follow your defined process (orient → outline → write → self-review → commit + push → update issue → report). Plans commit directly to main per R5.5.1.
```

If `$ARGUMENTS` is empty, ask the maintainer "Which `agent:plan` issue would you like to plan?" and use their answer.

Do not edit any files yourself (R4.4).
