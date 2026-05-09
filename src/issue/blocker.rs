use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{NotFoundError, UserError};
use crate::storage::comment_file::{
    CommentFile, CommentFrontmatter, comment_path, next_comment_seq, write_comment,
};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_filename, iso_frontmatter, now_utc};

const TERMINAL_STATES: &[&str] = &["done", "dropped"];
const BLOCKER_VOCAB: &[&str] = &["maintainer-input", "external", "upstream"];

pub struct SetArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub blocker: String,
    pub comment: Option<String>,
}

pub struct ClearArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub comment: Option<String>,
}

pub fn run_set(args: SetArgs) -> Result<()> {
    if !BLOCKER_VOCAB.contains(&args.blocker.as_str()) {
        return Err(UserError(format!(
            "invalid blocker '{}'; expected one of: {}",
            args.blocker,
            BLOCKER_VOCAB.join(", ")
        ))
        .into());
    }

    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }

    let now = now_utc();
    let created_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);
    let comments_dir = paths.comments_dir(args.id);

    let seq = with_repo_lock(&paths, || {
        let mut issue = read_issue(&issue_path)?;
        if TERMINAL_STATES.contains(&issue.frontmatter.state.as_str()) {
            return Err(UserError(format!(
                "issue #{} is in terminal state '{}'; blocker cannot be set",
                args.id, issue.frontmatter.state
            ))
            .into());
        }

        let seq = next_comment_seq(&comments_dir)?;
        let body = args.comment.clone().unwrap_or_else(|| {
            format!("Blocker set: {}", args.blocker)
        });
        let fm = CommentFrontmatter {
            seq,
            issue: args.id,
            author: args.actor.clone(),
            kind: "blocker-set".to_string(),
            created: created_iso.clone(),
            from: None,
            to: None,
            blocker: Some(args.blocker.clone()),
        };
        write_comment(
            &comment_path(&comments_dir, seq, &filename_iso, &args.actor),
            &CommentFile { frontmatter: fm, body },
        )?;

        issue.frontmatter.blocker = Some(args.blocker.clone());
        issue.frontmatter.updated = created_iso.clone();
        write_issue(&issue_path, &issue)?;
        Ok(seq)
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"blocker\":\"{}\",\"seq\":{seq}}}",
            args.id, args.blocker
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Issue #{}: blocker set to {}", args.id, args.blocker);
    }
    Ok(())
}

pub fn run_clear(args: ClearArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }

    let now = now_utc();
    let created_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);
    let comments_dir = paths.comments_dir(args.id);

    let (seq, prior) = with_repo_lock(&paths, || {
        let mut issue = read_issue(&issue_path)?;
        let prior = match issue.frontmatter.blocker.clone() {
            Some(b) => b,
            None => {
                return Err(UserError(format!(
                    "issue #{} has no blocker set",
                    args.id
                ))
                .into());
            }
        };

        let seq = next_comment_seq(&comments_dir)?;
        let body = args.comment.clone().unwrap_or_else(|| {
            format!("Blocker cleared: {prior}")
        });
        let fm = CommentFrontmatter {
            seq,
            issue: args.id,
            author: args.actor.clone(),
            kind: "blocker-cleared".to_string(),
            created: created_iso.clone(),
            from: None,
            to: None,
            blocker: Some(prior.clone()),
        };
        write_comment(
            &comment_path(&comments_dir, seq, &filename_iso, &args.actor),
            &CommentFile { frontmatter: fm, body },
        )?;

        issue.frontmatter.blocker = None;
        issue.frontmatter.updated = created_iso.clone();
        write_issue(&issue_path, &issue)?;
        Ok((seq, prior))
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"cleared\":\"{prior}\",\"seq\":{seq}}}",
            args.id
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Issue #{}: blocker cleared (was {prior})", args.id);
    }
    Ok(())
}
