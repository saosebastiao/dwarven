---
id: 24
title: 'rustdoc: crate root + small modules'
type: doc
state: done
priority: p2
epic: code-docs
created: 2026-05-11T15:54:40Z
created_by: maintainer
updated: 2026-05-11T16:05:58Z
---
Crate-level docs and the smaller modules left out of the previous three rustdoc issues.

Files in scope:
- src/lib.rs — crate-level //! describing the binary's structure, the library reuse pattern, pointers to docs/architecture/overview.md
- src/main.rs — file-level //! on the binary entry point; clap structure overview
- src/init.rs — dwarven init implementation
- src/config.rs — dwarven config get/set CLI (distinct from daemon::config and scheduler::config)
- src/schedule_cli.rs — dwarven schedule next formatter
- src/time.rs — ISO-8601 helpers
- src/eval/mod.rs — already has //! header; extend with /// on the re-exports
- src/eval/scenario.rs, matcher.rs, mock_tools.rs — pub types and functions

Aim: bring crate-level rustdoc coverage to the point where `cargo doc --no-deps` produces a usable browsing experience as the entry point for someone learning the codebase. Cross-link to the architecture docs (intra-doc links to URLs are fine).
