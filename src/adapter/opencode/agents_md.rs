//! `.opencode/AGENTS.md` — orientation copy auto-loaded into the
//! model's context at session start (via the `instructions` reference
//! in `opencode.json`). Substitutes the SessionStart hook Claude Code
//! has; see `host-adapter.md#R4.5`.

pub fn render() -> String {
    let mut out = String::new();
    out.push_str("# Dwarven on opencode\n\n");
    out.push_str("This repository uses [Dwarven](https://github.com/danieltoone/dwarven), a host-agnostic specification-driven development system. The opencode adapter (`docs/specs/host-adapter.md#R4`) materializes the ten agents below as opencode subagents.\n\n");

    out.push_str("## Dispatching agents\n\n");
    out.push_str("Use `@<name>` mentions to dispatch:\n\n");
    out.push_str("| Mention | Role |\n|---|---|\n");
    out.push_str("| `@spec` | edit `docs/specs/*.md` in dialogue |\n");
    out.push_str("| `@architect` | author `docs/architecture/*.md` |\n");
    out.push_str("| `@gap` | scan for spec-vs-code divergence |\n");
    out.push_str("| `@pm` | decompose `state: pm` issues |\n");
    out.push_str("| `@plan` | author implementation plan |\n");
    out.push_str("| `@test` | write failing tests on `feat/<id>-<slug>` |\n");
    out.push_str("| `@implement` | make tests pass |\n");
    out.push_str("| `@review` | diff vs main, approve+merge or request changes |\n");
    out.push_str("| `@doc` | post-merge docs + CHANGELOG |\n");
    out.push_str("| `@triage` | audit the open queue |\n\n");

    out.push_str("## Hub daemon\n\n");
    out.push_str("Dwarven includes a local coordination hub. To start it: `dwarven serve` (foreground) or `dwarven serve &` (detached). To check status from this shell: `dwarven daemon status`.\n\n");
    out.push_str("The web UI is at `http://127.0.0.1:7777` when the daemon is running. The maintainer shell here is read-only; mutations route through dispatched agents.\n\n");

    out.push_str("## What's different from Claude Code\n\n");
    out.push_str("- Dispatch syntax is `@<name>` rather than `/<name>`.\n");
    out.push_str("- The `question` primitive is free-text; dialogue agents render alternatives as Markdown in their response and the maintainer answers in chat (`host-adapter.md#R4.4`).\n");
    out.push_str("- No SessionStart / PreToolUse hooks. The universal-deny list (`agent-roster.md#R13`) is enforced via the global `opencode.json` permission floor + materialize-time validation (`host-adapter.md#R4.8`).\n\n");

    out.push_str("## Pointers\n\n");
    out.push_str("- `docs/specs/dwarven.md` — top-level identity.\n");
    out.push_str("- `docs/specs/agent-roster.md` — per-agent contracts.\n");
    out.push_str("- `docs/specs/host-adapter.md#R4` — opencode adapter spec.\n");
    out.push_str("- `docs/specs/work-states.md` — the state machine.\n");
    out.push_str("- `docs/specs/dwarven-cli.md` — CLI surface.\n");
    out
}
