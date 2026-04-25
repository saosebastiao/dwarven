---
description: "Dispatch the Specification Gap Analysis agent to scan for spec gaps (per R3.3)"
---

Use the `Agent` tool with `subagent_type="specification-gap-analysis"` to dispatch the Gap Analysis agent.

Pass this prompt:

```
The maintainer invoked /gap with scope: $ARGUMENTS

Follow your defined process (orient → inventory evidence → scan → dedupe → file new gap issues → report). Per R3.3, you are read-only on the filesystem — you discover gaps as GitHub issues, you do not fix them.
```

If `$ARGUMENTS` is empty, scan the full spec.

Do not edit any files yourself (R4.4).
