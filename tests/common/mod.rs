// Each `tests/*.rs` file is its own crate; helpers used by some files but not
// all otherwise trigger dead_code warnings in the unused crates.
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;
use tempfile::TempDir;

/// Build a `dwarven` invocation rooted at the given path.
pub fn dwarven(repo: &Path) -> Command {
    let mut cmd = Command::cargo_bin("dwarven").expect("dwarven binary built");
    cmd.arg("--repo").arg(repo);
    // Detach the test from the user's environment so DWARVEN_ACTOR doesn't
    // leak into the assertions.
    cmd.env_remove("DWARVEN_ACTOR");
    cmd
}

/// Initialize a fresh tempdir as a dwarven repo. Returns the tempdir handle
/// (drop closes it) so callers retain ownership.
pub fn fresh_repo() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let output = dwarven(tmp.path())
        .args(["init"])
        .output()
        .expect("init runs");
    assert!(
        output.status.success(),
        "dwarven init failed in fresh_repo: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    tmp
}

/// Read a file under the repo's `.dwarven/` (relative path).
pub fn read_dwarven(repo: &Path, rel: &str) -> String {
    std::fs::read_to_string(repo.join(".dwarven").join(rel))
        .unwrap_or_else(|e| panic!("reading .dwarven/{rel}: {e}"))
}
