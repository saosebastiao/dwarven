//! opencode adapter materialization per `host-adapter.md#R4`.
//!
//! Output surface:
//! - `opencode.json` at repo root (R4.8 global permission floor + instructions ref)
//! - `.opencode/AGENTS.md` (R4.5 orientation copy)
//! - `.opencode/agents/<name>.md` × 10 (R4.1 dispatched subagents)
//! - `.opencode/agents/build.md` (R4.3 maintainer primary agent)

pub mod agents;
pub mod agents_md;
pub mod maintainer;
pub mod opencode_json;
pub mod validation;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::adapter::ChangeSummary;
use crate::adapter::registry;
use crate::storage::write_atomic;

pub fn install(repo_root: &Path) -> Result<ChangeSummary> {
    let opencode_dir = repo_root.join(".opencode");
    let agents_dir = opencode_dir.join("agents");
    fs::create_dir_all(&agents_dir)
        .with_context(|| format!("creating {}", agents_dir.display()))?;

    let mut summary = ChangeSummary {
        written: 0,
        unchanged: 0,
    };

    // R4.8.3: materialize-time validation. Reject if any agent's allow
    // patterns would shadow a global deny pattern.
    validation::check_no_shadowing(registry::roster(), opencode_json::deny_patterns())
        .context("validating opencode adapter materialization")?;

    // 1. opencode.json at repo root (global permission floor + instructions ref).
    write_if_changed(
        &repo_root.join("opencode.json"),
        &opencode_json::render(),
        &mut summary,
    )?;

    // 2. .opencode/AGENTS.md (orientation).
    write_if_changed(
        &opencode_dir.join("AGENTS.md"),
        &agents_md::render(),
        &mut summary,
    )?;

    // 3. Dispatched subagent files.
    for agent in registry::roster() {
        let path = agents_dir.join(format!("{}.md", agent.name));
        write_if_changed(&path, &agents::render(agent), &mut summary)?;
    }

    // 4. Maintainer primary agent.
    write_if_changed(
        &agents_dir.join("build.md"),
        &maintainer::render(),
        &mut summary,
    )?;

    Ok(summary)
}

fn write_if_changed(path: &Path, content: &str, summary: &mut ChangeSummary) -> Result<()> {
    if read_if_exists(path)?.as_deref() == Some(content) {
        summary.unchanged += 1;
        return Ok(());
    }
    write_atomic(path, content.as_bytes())?;
    summary.written += 1;
    Ok(())
}

fn read_if_exists(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}
