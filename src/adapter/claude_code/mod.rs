//! Claude Code host adapter.
//!
//! Materializes the host-agnostic [`crate::adapter::registry::ROSTER`]
//! into the Claude Code on-disk surface:
//!
//! - `.claude/agents/<name>.md` (one per agent; frontmatter declares
//!   tool allowlist).
//! - `.claude/commands/<name>.md` (slash commands).
//! - `.claude/settings.json` (project-wide allow + R13 deny + hooks).
//! - `.claude/hooks/{session-start,pre-tool-use}.sh`.
//!
//! Enforcement strategy: settings allowlist is the first line; the
//! `pre-tool-use.sh` hook is a runtime second line of defense against
//! any pattern that would slip through.
//!
//! See [`docs/architecture/host-adapter.md`](../../../../docs/architecture/host-adapter.md).

pub mod agents;
pub mod commands;
pub mod hooks;
pub mod settings;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::adapter::ChangeSummary;
use crate::storage::write_atomic;

/// `dwarven init --host claude-code` entry point. Materializes the
/// Claude Code adapter into `repo_root` and returns a counter of
/// files written vs. unchanged.
///
/// Idempotent: re-running with the same registry state writes no files
/// (every output matches what's already on disk).
pub fn install(repo_root: &Path) -> Result<ChangeSummary> {
    let claude_dir = repo_root.join(".claude");
    let agents_dir = claude_dir.join("agents");
    let commands_dir = claude_dir.join("commands");
    let hooks_dir = claude_dir.join("hooks");
    fs::create_dir_all(&agents_dir).with_context(|| format!("creating {}", agents_dir.display()))?;
    fs::create_dir_all(&commands_dir)
        .with_context(|| format!("creating {}", commands_dir.display()))?;
    fs::create_dir_all(&hooks_dir).with_context(|| format!("creating {}", hooks_dir.display()))?;

    let mut summary = ChangeSummary {
        written: 0,
        unchanged: 0,
    };

    for agent in agents::roster() {
        let path = agents_dir.join(format!("{}.md", agent.name));
        write_if_changed(&path, &agents::render(agent), &mut summary)?;
    }

    for cmd in commands::list() {
        let path = commands_dir.join(format!("{}.md", cmd.name));
        write_if_changed(&path, &commands::render(&cmd), &mut summary)?;
    }

    write_if_changed(
        &claude_dir.join("settings.json"),
        &settings::render(),
        &mut summary,
    )?;

    let session_start = hooks_dir.join("session-start.sh");
    write_executable_if_changed(&session_start, &hooks::session_start_script(), &mut summary)?;
    let pre_tool_use = hooks_dir.join("pre-tool-use.sh");
    write_executable_if_changed(&pre_tool_use, &hooks::pre_tool_use_script(), &mut summary)?;

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

#[cfg(unix)]
fn write_executable_if_changed(
    path: &Path,
    content: &str,
    summary: &mut ChangeSummary,
) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    write_if_changed(path, content, summary)?;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_executable_if_changed(
    path: &Path,
    content: &str,
    summary: &mut ChangeSummary,
) -> Result<()> {
    write_if_changed(path, content, summary)
}

fn read_if_exists(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

#[allow(dead_code)] // exposed for tests
pub fn agents_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".claude").join("agents")
}
