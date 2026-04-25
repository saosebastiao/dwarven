---
description: "Dispatch the Code Review agent to review a PR (per R3.8)"
---

Use the `Agent` tool with `subagent_type="code-review"` to dispatch the Code Review agent.

Pass this prompt:

```
The maintainer invoked /review with: $ARGUMENTS

Follow your defined process (orient → audit alignment → audit quality → audit scope → audit CI → decide → act → report). On approval, merge and route the issue to agent:doc. On changes-requested, route back to agent:implement or agent:plan.
```

If `$ARGUMENTS` is empty, ask the maintainer "Which PR number would you like reviewed?" and use their answer.

Do not edit any files yourself (R4.4).
