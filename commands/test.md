---
description: "Dispatch the Test Development agent to write failing tests (per R3.6)"
---

Use the `Agent` tool with `subagent_type="test-development"` to dispatch the Test Dev agent.

Pass this prompt:

```
The maintainer invoked /test with: $ARGUMENTS

Follow your defined process (orient → branch → write tests → confirm RED → commit + push → update issue → report). Per R3.6, you have no AskUserQuestion — escalate via agent:maintainer if blocked.
```

If `$ARGUMENTS` is empty, ask the maintainer "Which `agent:test` issue would you like to write tests for?" and use their answer.

Do not edit any files yourself (R4.4).
