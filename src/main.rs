use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgGroup, Args, Parser, Subcommand};

mod init;
mod issue;
mod storage;
mod time;

use issue::comment::CommentArgs;
use issue::create::{BodyInput, CreateArgs, classify_exit_code};
use issue::list::{ListArgs, SortField};
use issue::transition::TransitionArgs;
use issue::view::ViewArgs;

#[derive(Parser)]
#[command(name = "dwarven", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Operate on the repository rooted at <path>; defaults to current directory.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    /// Attribution for any mutation; defaults to env DWARVEN_ACTOR, then "maintainer".
    #[arg(long, global = true)]
    actor: Option<String>,

    /// Suppress non-essential stdout.
    #[arg(long, global = true)]
    quiet: bool,

    /// Machine-parseable JSON output.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize .dwarven/ in the current directory.
    Init {
        /// Host adapter to install (e.g., claude-code). May be repeated. Not yet implemented.
        #[arg(long)]
        host: Vec<String>,
    },

    /// Issue management.
    Issue {
        #[command(subcommand)]
        action: IssueAction,
    },
}

#[derive(Subcommand)]
enum IssueAction {
    /// Create a new issue.
    Create(IssueCreateArgs),
    /// View an issue and its comments.
    View(IssueViewArgs),
    /// List issues, optionally filtered.
    List(IssueListArgs),
    /// Add a comment to an issue.
    Comment(IssueCommentArgs),
    /// Transition an issue to a new state.
    Transition(IssueTransitionArgs),
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("body_input")
        .args(["body", "body_file", "body_stdin"])
        .multiple(false)
))]
struct IssueCreateArgs {
    /// Issue type: spec-gap, feature, bug, arch, doc, chore.
    #[arg(long)]
    r#type: String,

    /// Issue title (≤120 bytes, non-empty after trim).
    #[arg(long)]
    title: String,

    /// Inline body text.
    #[arg(long)]
    body: Option<String>,

    /// Read body from a file.
    #[arg(long, value_name = "PATH")]
    body_file: Option<PathBuf>,

    /// Read body from stdin.
    #[arg(long)]
    body_stdin: bool,

    /// Override the type-default initial state.
    #[arg(long)]
    state: Option<String>,

    /// Maintainer-asserted priority: p0, p1, p2.
    #[arg(long)]
    priority: Option<String>,

    /// Comma-separated list of issue IDs that block this one.
    #[arg(long, value_delimiter = ',')]
    blocked_by: Vec<u64>,

    /// Comma-separated list of issue IDs that this one blocks.
    #[arg(long, value_delimiter = ',')]
    blocks: Vec<u64>,

    /// Epic slug (kebab-case).
    #[arg(long)]
    epic: Option<String>,
}

#[derive(Args)]
struct IssueViewArgs {
    /// Issue id.
    id: u64,

    /// Suppress comments; print issue header + body only.
    #[arg(long)]
    no_comments: bool,

    /// Filter comments to state-change records only.
    #[arg(long)]
    state_history: bool,

    /// Show only the most recent N comments.
    #[arg(long, value_name = "N")]
    last: Option<usize>,
}

#[derive(Args)]
struct IssueListArgs {
    /// Comma-separated states to include (e.g. plan,test).
    #[arg(long, value_delimiter = ',')]
    state: Vec<String>,

    /// Comma-separated types to include.
    #[arg(long = "type", value_delimiter = ',')]
    types: Vec<String>,

    /// Comma-separated blockers to include.
    #[arg(long, value_delimiter = ',')]
    blocker: Vec<String>,

    /// Comma-separated priorities to include (use 'unset' to match issues with no priority).
    #[arg(long, value_delimiter = ',')]
    priority: Vec<String>,

    /// Restrict to a single epic slug.
    #[arg(long)]
    epic: Option<String>,

    /// Show only active issues (default).
    #[arg(long)]
    open: bool,

    /// Show only terminal-state issues (done, dropped).
    #[arg(long)]
    closed: bool,

    /// Show all issues regardless of state.
    #[arg(long)]
    all: bool,

    /// Literal-string filter against title and body.
    #[arg(long)]
    grep: Option<String>,

    /// Sort field: id, created, updated, priority. Default: updated.
    #[arg(long, default_value = "updated")]
    sort: String,
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("comment_body_input")
        .args(["body", "body_file", "body_stdin"])
        .multiple(false)
        .required(true)
))]
struct IssueCommentArgs {
    /// Issue id.
    id: u64,

    /// Inline body text.
    #[arg(long)]
    body: Option<String>,

    /// Read body from a file.
    #[arg(long, value_name = "PATH")]
    body_file: Option<PathBuf>,

    /// Read body from stdin.
    #[arg(long)]
    body_stdin: bool,
}

#[derive(Args)]
struct IssueTransitionArgs {
    /// Issue id.
    id: u64,

    /// Target state.
    new_state: String,

    /// Optional rationale recorded as the body of the state-change comment.
    #[arg(long)]
    comment: Option<String>,

    /// Force a transition not in the work-states.md graph. Maintainer-only;
    /// cannot transition into terminal states.
    #[arg(long)]
    r#override: bool,
}

fn main() {
    let cli = Cli::parse();

    let repo_root = match cli.repo.clone() {
        Some(p) => p,
        None => match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: cannot read current directory: {e}");
                std::process::exit(2);
            }
        },
    };

    let actor = resolve_actor(cli.actor.clone());

    let result: Result<i32> = match cli.command {
        Command::Init { host } => init::run(&repo_root, &host, cli.quiet).map(|_| 0),
        Command::Issue { action } => match action {
            IssueAction::Create(args) => {
                let body = if let Some(s) = args.body {
                    BodyInput::Inline(s)
                } else if let Some(p) = args.body_file {
                    BodyInput::File(p)
                } else if args.body_stdin {
                    BodyInput::Stdin
                } else {
                    BodyInput::None
                };
                let create_args = CreateArgs {
                    repo_root,
                    actor,
                    quiet: cli.quiet,
                    json: cli.json,
                    issue_type: args.r#type,
                    title: args.title,
                    body,
                    state: args.state,
                    priority: args.priority,
                    blocked_by: args.blocked_by,
                    blocks: args.blocks,
                    epic: args.epic,
                };
                map_issue(issue::create::run(create_args))
            }
            IssueAction::View(args) => {
                let view_args = ViewArgs {
                    repo_root,
                    id: args.id,
                    no_comments: args.no_comments,
                    state_history: args.state_history,
                    last: args.last,
                };
                map_issue(issue::view::run(view_args))
            }
            IssueAction::Comment(args) => {
                let body = if let Some(s) = args.body {
                    BodyInput::Inline(s)
                } else if let Some(p) = args.body_file {
                    BodyInput::File(p)
                } else if args.body_stdin {
                    BodyInput::Stdin
                } else {
                    BodyInput::None
                };
                let comment_args = CommentArgs {
                    repo_root,
                    actor,
                    quiet: cli.quiet,
                    json: cli.json,
                    id: args.id,
                    body,
                };
                map_issue(issue::comment::run(comment_args))
            }
            IssueAction::Transition(args) => {
                let trans_args = TransitionArgs {
                    repo_root,
                    actor,
                    quiet: cli.quiet,
                    json: cli.json,
                    id: args.id,
                    new_state: args.new_state,
                    comment: args.comment,
                    override_graph: args.r#override,
                };
                map_issue(issue::transition::run(trans_args))
            }
            IssueAction::List(args) => match SortField::parse(&args.sort) {
                Ok(sort) => {
                    let list_args = ListArgs {
                        repo_root,
                        states: opt_vec(args.state),
                        types: opt_vec(args.types),
                        blockers: opt_vec(args.blocker),
                        priorities: opt_vec(args.priority),
                        epic: args.epic,
                        open: args.open,
                        closed: args.closed,
                        all: args.all,
                        grep: args.grep,
                        sort,
                    };
                    map_issue(issue::list::run(list_args))
                }
                Err(e) => {
                    let code = classify_exit_code(&e);
                    eprintln!("error: {e:#}");
                    Ok(code)
                }
            },
        },
    };

    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}

fn map_issue(r: Result<()>) -> Result<i32> {
    match r {
        Ok(()) => Ok(0),
        Err(e) => {
            let code = classify_exit_code(&e);
            eprintln!("error: {e:#}");
            Ok(code)
        }
    }
}

fn opt_vec(v: Vec<String>) -> Option<Vec<String>> {
    if v.is_empty() { None } else { Some(v) }
}

fn resolve_actor(flag: Option<String>) -> String {
    if let Some(a) = flag {
        return a;
    }
    if let Ok(a) = std::env::var("DWARVEN_ACTOR") {
        if !a.is_empty() {
            return a;
        }
    }
    "maintainer".to_string()
}
