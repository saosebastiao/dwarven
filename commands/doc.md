---
description: "Dispatch the System Documentation agent to update docs after merge (per R3.9)"
---

Use the `Agent` tool with `subagent_type="system-documentation"` to dispatch the Documentation agent.

Pass this prompt:

```
The maintainer invoked /doc with: $ARGUMENTS

Follow your defined process (orient → identify doc impact → update docs → append CHANGELOG → self-review → commit + push → close issue → report). Docs describe what the code DOES; specs describe what it SHOULD do (R2.1).
```

If `$ARGUMENTS` is empty, ask the maintainer "Which `agent:doc` issue would you like to document?" and use their answer.

Do not edit any files yourself (R4.4).
