use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{BodyInput, NotFoundError, UserError, read_body};
use crate::storage::comment_file::{
    CommentFile, CommentFrontmatter, comment_path, next_comment_seq, write_comment,
};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_filename, iso_frontmatter, now_utc};

const TERMINAL_STATES: &[&str] = &["done", "dropped"];

pub struct CloseArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub dropped: bool,
    pub body: BodyInput,
}

pub fn run(args: CloseArgs) -> Result<()> {
    if matches!(args.body, BodyInput::None) {
        return Err(UserError(
            "close requires a closure comment via --comment, --comment-file, or --comment-stdin"
                .into(),
        )
        .into());
    }
    let body = read_body(&args.body)?;
    let target_state = if args.dropped { "dropped" } else { "done" };

    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }

    let now = now_utc();
    let created_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);
    let comments_dir = paths.comments_dir(args.id);

    let (seq, from_state) = with_repo_lock(&paths, || {
        let mut issue = read_issue(&issue_path)?;
        let from_state = issue.frontmatter.state.clone();

        if TERMINAL_STATES.contains(&from_state.as_str()) {
            return Err(UserError(format!(
                "issue #{} is already in terminal state '{from_state}'",
                args.id
            ))
            .into());
        }

        let seq = next_comment_seq(&comments_dir)?;

        let fm = CommentFrontmatter {
            seq,
            issue: args.id,
            author: args.actor.clone(),
            kind: "state-change".to_string(),
            created: created_iso.clone(),
            from: Some(from_state.clone()),
            to: Some(target_state.to_string()),
            blocker: None,
        };
        let path = comment_path(&comments_dir, seq, &filename_iso, &args.actor);
        write_comment(
            &path,
            &CommentFile {
                frontmatter: fm,
                body: body.clone(),
            },
        )?;

        issue.frontmatter.state = target_state.to_string();
        issue.frontmatter.updated = created_iso.clone();
        write_issue(&issue_path, &issue)?;

        Ok((seq, from_state))
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"from\":\"{from_state}\",\"to\":\"{target_state}\",\"seq\":{seq}}}",
            args.id
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!(
            "Closed issue #{}: {from_state} → {target_state}",
            args.id
        );
    }

    Ok(())
}
