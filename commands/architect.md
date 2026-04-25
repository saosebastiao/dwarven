---
description: "Dispatch the Architect agent to evolve docs/architecture/*.md (per R3.2)"
---

Use the `Agent` tool with `subagent_type="architect"` to dispatch the Architect agent.

Pass this prompt to the subagent:

```
The maintainer invoked /architect with: $ARGUMENTS

Follow your defined process (orient → dialogue → alternatives → write → self-review → resolve arch issues → commit + push → report). Per R3.2, dialogue freely with the maintainer using AskUserQuestion or plain text — you are dispatched interactively.
```

If `$ARGUMENTS` is empty, first ask the maintainer (in this shell, not via dispatch) "What architectural question would you like to work on?" and use their answer as `$ARGUMENTS`.

Do not edit any files yourself. Per R4.4, the shell never mutates state — your only role is to dispatch and relay the agent's result.
