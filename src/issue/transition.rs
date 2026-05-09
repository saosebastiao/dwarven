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
const ACTIVE_STATES: &[&str] = &[
    "spec",
    "architect",
    "pm",
    "plan",
    "test",
    "implement",
    "review",
    "doc",
    "maintainer",
];

/// Permitted transitions from `work-states.md#R6.2`. `dropped` is reachable
/// from any active state per R6.2.2 and is checked separately.
fn permitted_to(from: &str) -> &'static [&'static str] {
    match from {
        "spec" => &["done", "maintainer"],
        "architect" => &["done", "maintainer"],
        "pm" => &["plan", "done", "maintainer"],
        "plan" => &["test", "maintainer"],
        "test" => &["implement", "maintainer"],
        "implement" => &["review", "maintainer"],
        "review" => &["doc", "implement", "plan", "maintainer"],
        "doc" => &["done", "maintainer"],
        "maintainer" => ACTIVE_STATES,
        _ => &[],
    }
}

pub struct TransitionArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub new_state: String,
    pub comment: Option<String>,
    pub override_graph: bool,
}

pub fn run(args: TransitionArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }

    if args.override_graph {
        // R6.5.3: only the maintainer may use --override.
        if args.actor != "maintainer" {
            return Err(UserError(
                "--override is restricted to --actor maintainer".into(),
            )
            .into());
        }
        // R6.3.2: --override may not transition into terminal states.
        if TERMINAL_STATES.contains(&args.new_state.as_str()) {
            return Err(UserError(format!(
                "--override cannot transition into terminal state '{}'; use `dwarven issue close`",
                args.new_state
            ))
            .into());
        }
        if !ACTIVE_STATES.contains(&args.new_state.as_str()) {
            return Err(UserError(format!(
                "invalid target state '{}'; expected an active state: {}",
                args.new_state,
                ACTIVE_STATES.join(", ")
            ))
            .into());
        }
    }

    let now = now_utc();
    let created_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);
    let comments_dir = paths.comments_dir(args.id);

    let (seq, from_state) = with_repo_lock(&paths, || {
        let mut issue = read_issue(&issue_path)?;
        let from_state = issue.frontmatter.state.clone();

        // R7.4: terminal states are absorbing.
        if TERMINAL_STATES.contains(&from_state.as_str()) {
            return Err(UserError(format!(
                "issue #{} is in terminal state '{from_state}'; transitions out are forbidden",
                args.id
            ))
            .into());
        }

        if from_state == args.new_state {
            return Err(UserError(format!(
                "issue #{} is already in state '{from_state}'",
                args.id
            ))
            .into());
        }

        if !args.override_graph {
            let dropped_ok = args.new_state == "dropped"
                && ACTIVE_STATES.contains(&from_state.as_str());
            let in_graph = permitted_to(&from_state).contains(&args.new_state.as_str());
            if !dropped_ok && !in_graph {
                let mut allowed: Vec<&str> = permitted_to(&from_state).to_vec();
                if ACTIVE_STATES.contains(&from_state.as_str()) {
                    allowed.push("dropped");
                }
                return Err(UserError(format!(
                    "illegal transition '{from_state}' → '{}'; permitted: {}",
                    args.new_state,
                    allowed.join(", ")
                ))
                .into());
            }
        }

        let seq = next_comment_seq(&comments_dir)?;

        let fm = CommentFrontmatter {
            seq,
            issue: args.id,
            author: args.actor.clone(),
            kind: "state-change".to_string(),
            created: created_iso.clone(),
            from: Some(from_state.clone()),
            to: Some(args.new_state.clone()),
            blocker: None,
        };
        let body = args
            .comment
            .clone()
            .unwrap_or_else(|| format!("Transition {} → {}", from_state, args.new_state));
        let path = comment_path(&comments_dir, seq, &filename_iso, &args.actor);
        write_comment(
            &path,
            &CommentFile {
                frontmatter: fm,
                body,
            },
        )?;

        issue.frontmatter.state = args.new_state.clone();
        issue.frontmatter.updated = created_iso.clone();
        write_issue(&issue_path, &issue)?;

        Ok((seq, from_state))
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"from\":\"{from_state}\",\"to\":\"{}\",\"seq\":{seq}}}",
            args.id, args.new_state
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!(
            "Issue #{}: {from_state} → {}",
            args.id, args.new_state
        );
    }

    Ok(())
}
