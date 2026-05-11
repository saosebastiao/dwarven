//! `dwarven init` — scaffold `.dwarven/` and optionally materialize
//! one or more host adapters.
//!
//! Idempotent. Re-running `init` against an existing `.dwarven/`
//! directory leaves the canonical files alone; re-running `--host <h>`
//! against an already-installed adapter is a no-op.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

use crate::adapter;
use crate::storage::write_atomic;

const GITIGNORE_BODY: &str = "\
# Derived state owned by the dwarven daemon. Per coordination-hub.md#R8.4
# the SQLite index is single-machine and rebuilt from the canonical files
# in this directory; it must not be committed.
.index.sqlite
.index.sqlite-*

# Daemon PID file. Per coordination-hub.md#R3.5.
.daemon.pid

# Lock file for the issue-id counter. Single-machine; never committed.
.config.lock
";

pub fn run(repo_root: &Path, hosts: &[String], quiet: bool) -> Result<()> {
    let dwarven_dir = repo_root.join(".dwarven");
    let config_path = dwarven_dir.join("config.toml");

    if dwarven_dir.exists() {
        report_existing(&dwarven_dir, &config_path, quiet)?;
        install_adapters(repo_root, hosts, quiet)?;
        return Ok(());
    }

    let repo_name = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dwarven-repo")
        .to_string();
    let repo_id = Uuid::new_v4();

    fs::create_dir_all(&dwarven_dir)
        .with_context(|| format!("creating {}", dwarven_dir.display()))?;
    fs::create_dir_all(dwarven_dir.join("issues"))
        .with_context(|| format!("creating {}/issues", dwarven_dir.display()))?;

    let config = render_config(&repo_id, &repo_name);
    write_atomic(&config_path, config.as_bytes())
        .with_context(|| format!("writing {}", config_path.display()))?;

    let gitignore_path = dwarven_dir.join(".gitignore");
    write_atomic(&gitignore_path, GITIGNORE_BODY.as_bytes())
        .with_context(|| format!("writing {}", gitignore_path.display()))?;

    if !quiet {
        println!(
            "Initialized dwarven at {}\n  repo id: {}\n  repo name: {}\n  next issue id: 1",
            dwarven_dir.display(),
            repo_id,
            repo_name,
        );
    }

    install_adapters(repo_root, hosts, quiet)?;
    Ok(())
}

fn install_adapters(repo_root: &Path, hosts: &[String], quiet: bool) -> Result<()> {
    for host in hosts {
        let summary = adapter::install(host, repo_root)
            .with_context(|| format!("installing adapter '{host}'"))?;
        if !quiet {
            println!(
                "Installed adapter '{host}': {} written, {} unchanged",
                summary.written, summary.unchanged
            );
        }
    }
    Ok(())
}

fn report_existing(dwarven_dir: &Path, config_path: &Path, quiet: bool) -> Result<()> {
    if !config_path.exists() {
        return Err(anyhow!(
            "{} exists but {} is missing; refusing to touch a partially-initialized directory",
            dwarven_dir.display(),
            config_path.display()
        ));
    }
    if quiet {
        return Ok(());
    }
    let raw = fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let parsed: toml::Value = raw
        .parse()
        .with_context(|| format!("parsing {}", config_path.display()))?;
    let repo_id = parsed
        .get("repo")
        .and_then(|t| t.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    let repo_name = parsed
        .get("repo")
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    let next_id = parsed
        .get("counters")
        .and_then(|t| t.get("next_issue_id"))
        .and_then(|v| v.as_integer())
        .unwrap_or(0);

    println!(
        "Dwarven already initialized at {}\n  repo id: {}\n  repo name: {}\n  next issue id: {}",
        dwarven_dir.display(),
        repo_id,
        repo_name,
        next_id,
    );
    Ok(())
}

fn render_config(repo_id: &Uuid, repo_name: &str) -> String {
    format!(
        "\
# Dwarven hub configuration. Schema: coordination-hub.md#R10.

[repo]
id = \"{repo_id}\"
name = \"{repo_name}\"

[counters]
next_issue_id = 1

[daemon]
port = 7777
bind = \"127.0.0.1\"
reconciliation_interval_seconds = 60

[scheduler]
alpha = 0.5

[scheduler.priority_weights]
p0 = 4
p1 = 2
p2 = 1
unset = 1

[triage]
stale_threshold_days = 14
"
    )
}

