use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};

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
