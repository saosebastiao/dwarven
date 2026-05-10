pub mod claude_code;

use std::path::Path;

use anyhow::Result;

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
        other => Err(anyhow::anyhow!(
            "unknown host adapter '{other}'; supported: claude-code"
        )),
    }
}
