use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{NotFoundError, UserError};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_frontmatter, now_utc};

const PRIORITIES: &[&str] = &["p0", "p1", "p2"];
const TERMINAL_STATES: &[&str] = &["done", "dropped"];

pub struct PriorityArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub priority: String,
}

pub struct ClearArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
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
        if TERMINAL_STATES.contains(&issue.frontmatter.state.as_str()) {
            return Err(UserError(format!(
                "issue #{} is in terminal state '{}'; priority cannot be changed",
                args.id, issue.frontmatter.state
            ))
            .into());
        }
        if issue.frontmatter.priority.is_none() {
            return Err(UserError(format!("issue #{} has no priority set", args.id)).into());
        }
        issue.frontmatter.priority = None;
        issue.frontmatter.updated = now_iso.clone();
        write_issue(&issue_path, &issue)
    })?;

    if !args.quiet && !args.json {
        println!("Issue #{}: priority cleared", args.id);
    }
    Ok(())
}

pub fn run(args: PriorityArgs) -> Result<()> {
    if !PRIORITIES.contains(&args.priority.as_str()) {
        return Err(UserError(format!(
            "invalid priority '{}'; expected one of: {}",
            args.priority,
            PRIORITIES.join(", ")
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
                "issue #{} is in terminal state '{}'; priority cannot be changed",
                args.id, issue.frontmatter.state
            ))
            .into());
        }
        issue.frontmatter.priority = Some(args.priority.clone());
        issue.frontmatter.updated = now_iso.clone();
        write_issue(&issue_path, &issue)
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"priority\":\"{}\"}}",
            args.id, args.priority
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Issue #{}: priority set to {}", args.id, args.priority);
    }
    Ok(())
}
