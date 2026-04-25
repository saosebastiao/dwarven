---
description: "Dispatch the Project Management agent to decompose an agent:pm issue (per R3.4)"
---

Use the `Agent` tool with `subagent_type="project-management"` to dispatch the PM agent.

Pass this prompt:

```
The maintainer invoked /pm with: $ARGUMENTS

Follow your defined process (orient → decompose → dialogue if needed → file children → update parent → report). Per R3.4, children get the WHAT only — Planning (R3.5) writes the HOW.
```

If `$ARGUMENTS` is empty, ask the maintainer "Which `agent:pm` issue would you like to decompose?" and use their answer.

Do not edit any files yourself (R4.4).
