//! `dwarven issue priority-override` — absolute scheduler rank override.
//!
//! Bypasses the dep-graph computation. Maintainer-only; in the R13
//! deny list for every host adapter. See [`crate::scheduler`] for the
//! algorithm and `dep-graph.md#R4` for the policy.

use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{NotFoundError, UserError};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_frontmatter, now_utc};

const TERMINAL_STATES: &[&str] = &["done", "dropped"];

pub struct SetArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub value: f64,
}

pub struct ClearArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
}

pub fn run_set(args: SetArgs) -> Result<()> {
    if !args.value.is_finite() {
        return Err(UserError(format!(
            "priority-override value must be a finite number (got {})",
            args.value
        ))
        .into());
    }

    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }
    let now_iso = iso_frontmatter(&now_utc());

    with_repo_lock(&paths, || {
        let mut issue = read_issue(&issue_path)?;
        if TERMINAL_STATES.contains(&issue.frontmatter.state.as_str()) {
            return Err(UserError(format!(
                "issue #{} is in terminal state '{}'; priority-override cannot be set",
                args.id, issue.frontmatter.state
            ))
            .into());
        }
        issue.frontmatter.effective_priority_override = Some(args.value);
        issue.frontmatter.updated = now_iso.clone();
        write_issue(&issue_path, &issue)
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"effective_priority_override\":{}}}",
            args.id, args.value
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!(
            "Issue #{}: priority-override set to {}",
            args.id, args.value
        );
    }
    Ok(())
}

pub fn run_clear(args: ClearArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }
    let now_iso = iso_frontmatter(&now_utc());

    with_repo_lock(&paths, || {
        let mut issue = read_issue(&issue_path)?;
        if issue.frontmatter.effective_priority_override.is_none() {
            return Err(UserError(format!(
                "issue #{} has no priority-override set",
                args.id
            ))
            .into());
        }
        issue.frontmatter.effective_priority_override = None;
        issue.frontmatter.updated = now_iso.clone();
        write_issue(&issue_path, &issue)
    })?;

    if !args.quiet && !args.json {
        println!("Issue #{}: priority-override cleared", args.id);
    } else if args.json {
        println!("{{\"issue\":{},\"ok\":true}}", args.id);
        eprintln!("warning: --json output schema is not yet stable");
    }
    Ok(())
}
