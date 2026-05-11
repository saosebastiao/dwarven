//! Atomic file writes via temp + rename.
//!
//! Every hub-tracked file in `.dwarven/` (issue.md, comment files,
//! config.toml, settings.json, adapter outputs) is written through
//! [`write_atomic`]. POSIX [`rename(2)`] within a single directory is
//! atomic: readers see either the old file or the complete new file,
//! never a partial write or a torn truncate. This is the load-bearing
//! invariant for file watchers (who must not emit reindex events on
//! mid-write state) and for the human-editing escape hatch (who must
//! not see half-written files in their editor).
//!
//! Architecture: [`docs/architecture/storage-layout.md`](../../../docs/architecture/storage-layout.md).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};

/// Write `bytes` to `target` atomically.
///
/// Writes to a sibling temp file `.<target-name>.tmp.<pid>` in the
/// same directory, then [`fs::rename`]s over the target. The same-directory
/// constraint is required: `rename(2)` is only atomic within a single
/// filesystem, and same-directory temp paths guarantee that regardless
/// of how `.dwarven/` is mounted.
///
/// The PID suffix disambiguates concurrent writes from the same process
/// (rare; same-process serialization is handled by the repo lock).
///
/// Errors if `target` has no parent directory, if the temp write fails,
/// or if the rename fails. Does not `fsync` the temp file — durability
/// guarantees are provided by the surrounding atomic-write convention
/// in callers that care (`coordination-hub.md#R8.3` does not require
/// crash-resistant durability for v1).
pub fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| anyhow!("target has no parent: {}", target.display()))?;
    let tmp = parent.join(format!(
        ".{}.tmp.{}",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("dwarven-write"),
        std::process::id()
    ));
    fs::write(&tmp, bytes).with_context(|| format!("writing temp {}", tmp.display()))?;
    fs::rename(&tmp, target)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), target.display()))?;
    Ok(())
}
