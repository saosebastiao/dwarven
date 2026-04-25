---
description: "Dispatch the Triage agent to audit issue queue hygiene (per R3.10)"
---

Use the `Agent` tool with `subagent_type="triage"` to dispatch the Triage agent.

Pass this prompt:

```
The maintainer invoked /triage with: $ARGUMENTS

Follow your defined process (inventory → audit labels → audit staleness → compose report → post report → report to maintainer). Per R3.10, you do not perform substantive work — only queue hygiene.
```

If `$ARGUMENTS` is empty, run a default triage scan over all open issues.

Do not edit any files yourself (R4.4).
