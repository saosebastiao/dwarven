---
description: "Dispatch the System Specification agent to evolve docs/specs/*.md (per R3.1)"
---

Use the `Agent` tool with `subagent_type="system-specification"` to dispatch the System Specification agent.

Pass this prompt to the subagent:

```
The maintainer invoked /spec with: $ARGUMENTS

Follow your defined process (orient → dialogue → alternatives → write → self-review → CHANGELOG → resolve gaps → commit + push → report). Per R3.1, dialogue freely with the maintainer using AskUserQuestion or plain text — you are dispatched interactively.
```

If `$ARGUMENTS` is empty, first ask the maintainer (in this shell, not via dispatch) "What aspect of the spec would you like to work on?" and use their answer as `$ARGUMENTS`.

Do not edit any files yourself. Per R4.4, the shell never mutates state — your only role here is to dispatch the agent and relay its result back.
