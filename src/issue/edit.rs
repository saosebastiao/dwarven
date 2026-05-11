//! `dwarven issue edit` — edit low-churn frontmatter fields.
//!
//! `--title`, `--type`, `--epic`. Body changes are out of scope; use
//! comments or direct file edits for those. Changing `--type` does
//! not re-route the state.

use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{NotFoundError, UserError, validate_epic, validate_title, validate_type};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_frontmatter, now_utc};

const TERMINAL_STATES: &[&str] = &["done", "dropped"];

pub struct EditArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub title: Option<String>,
    pub issue_type: Option<String>,
    pub epic: Option<String>,
}

pub fn run(args: EditArgs) -> Result<()> {
    if args.title.is_none() && args.issue_type.is_none() && args.epic.is_none() {
        return Err(UserError(
            "edit requires at least one of: --title, --type, --epic".into(),
        )
        .into());
    }
    if let Some(t) = &args.title {
        validate_title(t)?;
    }
    if let Some(ty) = &args.issue_type {
        validate_type(ty)?;
    }
    if let Some(e) = &args.epic {
        validate_epic(e)?;
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
                "issue #{} is in terminal state '{}'; edit forbidden",
                args.id, issue.frontmatter.state
            ))
            .into());
        }
        if let Some(t) = &args.title {
            issue.frontmatter.title = t.clone();
        }
        if let Some(ty) = &args.issue_type {
            issue.frontmatter.issue_type = ty.clone();
        }
        if let Some(e) = &args.epic {
            issue.frontmatter.epic = Some(e.clone());
        }
        issue.frontmatter.updated = now_iso.clone();
        write_issue(&issue_path, &issue)
    })?;

    if !args.quiet && !args.json {
        println!("Issue #{} updated.", args.id);
    } else if args.json {
        println!("{{\"issue\":{},\"ok\":true}}", args.id);
        eprintln!("warning: --json output schema is not yet stable");
    }
    Ok(())
}
