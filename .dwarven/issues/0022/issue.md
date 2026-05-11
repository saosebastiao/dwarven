---
id: 22
title: 'rustdoc: issue verbs + adapter modules'
type: doc
state: done
priority: p1
epic: code-docs
created: 2026-05-11T15:54:38Z
created_by: maintainer
updated: 2026-05-11T16:00:59Z
---
Add rustdoc to the per-CLI-verb issue modules and the host-adapter modules.

Files in scope:
- src/issue/mod.rs
- src/issue/create.rs, view.rs, list.rs, transition.rs, comment.rs, close.rs, blocker.rs, priority.rs, priority_override.rs, edit.rs, dep.rs
- src/adapter/mod.rs (install dispatch)
- src/adapter/registry.rs (AgentDef, Mode, ROSTER, roster)
- src/adapter/claude_code/{mod,agents,commands,settings,hooks}.rs
- src/adapter/opencode/{mod,agents,agents_md,maintainer,opencode_json,validation}.rs

For each pub item: one-line + the non-obvious bits. For the issue verbs that have an Args struct + run function, the doc should explain what the verb mutates, what locks it takes, what events it emits (if invoked via HTTP), and what error cases it returns. The validation module already has a strong //! header; extend it with /// on check_no_shadowing.
