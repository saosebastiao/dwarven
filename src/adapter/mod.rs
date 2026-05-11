//! Host adapter dispatch.
//!
//! [`install`] routes `dwarven init --host <h>` to the named host
//! module. Each host module (`claude_code`, `opencode`) renders the
//! host-agnostic agent roster (see [`registry::roster`]) into
//! host-specific files.
//!
//! Architecture: [`docs/architecture/host-adapter.md`](../../../docs/architecture/host-adapter.md).
//! Spec: [`docs/specs/host-adapter.md`](../../../docs/specs/host-adapter.md).

pub mod claude_code;
pub mod opencode;
pub mod registry;

use std::path::Path;

use anyhow::Result;

/// Counter returned by adapter installation. `written` counts files
/// whose content changed (or didn't previously exist); `unchanged`
/// counts files whose content matched the new render byte-for-byte.
///
/// `written == 0 && unchanged > 0` after a re-run indicates the
/// adapter is fully idempotent against the current registry state.
#[derive(Debug, Clone, Copy)]
pub struct ChangeSummary {
    pub written: usize,
    pub unchanged: usize,
}

/// Materialize the named host adapter into the repository at `repo_root`.
/// Returns the count of files written vs. unchanged so the caller can report
/// "no-op" for idempotent re-runs.
pub fn install(host: &str, repo_root: &Path) -> Result<ChangeSummary> {
    match host {
        "claude-code" => claude_code::install(repo_root),
        "opencode" => opencode::install(repo_root),
        other => Err(anyhow::anyhow!(
            "unknown host adapter '{other}'; supported: claude-code, opencode"
        )),
    }
}
