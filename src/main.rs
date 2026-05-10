use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgGroup, Args, Parser, Subcommand};

mod api;
mod config;
mod daemon;
mod index;
mod init;
mod issue;
mod storage;
mod time;

use config::{GetArgs as ConfigGetArgs, SetArgs as ConfigSetArgs};
use daemon::control::{RestartArgs, StatusArgs, StopArgs};
use daemon::serve::ServeArgs;
use index::ReindexArgs;
use issue::blocker::{ClearArgs as BlockerClearArgs, SetArgs as BlockerSetArgs};
use issue::close::CloseArgs;
use issue::comment::CommentArgs;
use issue::create::{BodyInput, CreateArgs, classify_exit_code};
use issue::dep::{AddArgs as DepAddArgs, RemoveArgs as DepRemoveArgs};
use issue::edit::EditArgs;
use issue::list::{ListArgs, SortField};
use issue::priority::PriorityArgs;
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

    /// Read or write `.dwarven/config.toml`.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Rebuild the SQLite index from `.dwarven/` files.
    Reindex,

    /// Run the coordination hub daemon in the foreground.
    Serve,

    /// Manage the running daemon (status / stop / restart).
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Print whether the daemon is running.
    Status,
    /// Send SIGTERM and wait for the daemon to exit.
    Stop,
    /// Stop the running daemon (if any) then start a new one in the foreground.
    Restart,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Read a config value by dotted key (e.g. `daemon.port`).
    Get(ConfigGetCli),
    /// Set a config value by dotted key.
    Set(ConfigSetCli),
}

#[derive(Args)]
struct ConfigGetCli {
    /// Dotted key path.
    key: String,
}

#[derive(Args)]
struct ConfigSetCli {
    /// Dotted key path.
    key: String,
    /// New value (parsed as int → float → bool → string).
    value: String,
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
    /// Close an issue terminally to `done` (default) or `dropped`.
    Close(IssueCloseArgs),
    /// Manage the issue's blocker field.
    Blocker {
        #[command(subcommand)]
        action: BlockerAction,
    },
    /// Set the issue's priority (p0/p1/p2). Maintainer-only by convention.
    Priority(IssuePriorityArgs),
    /// Edit low-churn frontmatter fields: --title, --type, --epic.
    Edit(IssueEditArgs),
    /// Manage dependency edges between issues.
    Dep {
        #[command(subcommand)]
        action: DepAction,
    },
}

#[derive(Subcommand)]
enum DepAction {
    /// Add an edge: `dwarven issue dep add <from> blocks <to>`.
    Add(DepAddCli),
    /// Remove an edge: `dwarven issue dep remove <from> <to>`.
    Remove(DepRemoveCli),
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("dep_rationale_input")
        .args(["rationale", "rationale_file", "rationale_stdin"])
        .multiple(false)
))]
struct DepAddCli {
    /// Source issue id (the one that does the blocking).
    from: u64,
    /// Literal word "blocks" (for readability — `dep add 5 blocks 7`).
    #[arg(value_parser = ["blocks"])]
    blocks: String,
    /// Target issue id (the one being blocked).
    to: u64,

    /// Inline rationale appended as a comment on the from-id issue.
    #[arg(long)]
    rationale: Option<String>,
    /// Read rationale from a file.
    #[arg(long, value_name = "PATH")]
    rationale_file: Option<PathBuf>,
    /// Read rationale from stdin.
    #[arg(long)]
    rationale_stdin: bool,
}

#[derive(Args)]
struct DepRemoveCli {
    /// Source issue id.
    from: u64,
    /// Target issue id.
    to: u64,
}

#[derive(Args)]
struct IssuePriorityArgs {
    /// Issue id.
    id: u64,
    /// Priority value: p0, p1, p2.
    priority: String,
}

#[derive(Args)]
struct IssueEditArgs {
    /// Issue id.
    id: u64,
    /// New title.
    #[arg(long)]
    title: Option<String>,
    /// New type (spec-gap/feature/bug/arch/doc/chore).
    #[arg(long = "type")]
    issue_type: Option<String>,
    /// New epic slug.
    #[arg(long)]
    epic: Option<String>,
}

#[derive(Subcommand)]
enum BlockerAction {
    /// Set the blocker on an issue.
    Set(BlockerSetCli),
    /// Clear the blocker on an issue.
    Clear(BlockerClearCli),
}

#[derive(Args)]
struct BlockerSetCli {
    /// Issue id.
    id: u64,
    /// Blocker value: maintainer-input, external, upstream.
    blocker: String,
    /// Optional rationale recorded as the blocker-set comment body.
    #[arg(long)]
    comment: Option<String>,
}

#[derive(Args)]
struct BlockerClearCli {
    /// Issue id.
    id: u64,
    /// Optional rationale recorded as the blocker-cleared comment body.
    #[arg(long)]
    comment: Option<String>,
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
#[command(group(
    ArgGroup::new("close_body_input")
        .args(["comment", "comment_file", "comment_stdin"])
        .multiple(false)
        .required(true)
))]
struct IssueCloseArgs {
    /// Issue id.
    id: u64,

    /// Close as `dropped` instead of the default `done`.
    #[arg(long)]
    dropped: bool,

    /// Inline closure comment.
    #[arg(long)]
    comment: Option<String>,

    /// Read closure comment from a file.
    #[arg(long, value_name = "PATH")]
    comment_file: Option<PathBuf>,

    /// Read closure comment from stdin.
    #[arg(long)]
    comment_stdin: bool,
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
        Command::Reindex => map_issue(index::run(ReindexArgs {
            repo_root,
            quiet: cli.quiet,
            json: cli.json,
        })),
        Command::Serve => match daemon::serve::run(ServeArgs {
            repo_root,
            quiet: cli.quiet,
        }) {
            Ok(()) => Ok(0),
            Err(e) => {
                let code = classify_exit_code(&e);
                eprintln!("error: {e:#}");
                Ok(code)
            }
        },
        Command::Daemon { action } => match action {
            DaemonAction::Status => match daemon::control::run_status(StatusArgs {
                repo_root,
                quiet: cli.quiet,
                json: cli.json,
            }) {
                Ok(code) => Ok(code),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    Ok(2)
                }
            },
            DaemonAction::Stop => match daemon::control::run_stop(StopArgs {
                repo_root,
                quiet: cli.quiet,
                json: cli.json,
            }) {
                Ok(code) => Ok(code),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    Ok(2)
                }
            },
            DaemonAction::Restart => match daemon::control::run_restart(RestartArgs {
                repo_root,
                quiet: cli.quiet,
            }) {
                Ok(code) => Ok(code),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    Ok(2)
                }
            },
        },
        Command::Config { action } => match action {
            ConfigAction::Get(args) => {
                let g = ConfigGetArgs {
                    repo_root,
                    quiet: cli.quiet,
                    json: cli.json,
                    key: args.key,
                };
                map_issue(config::run_get(g))
            }
            ConfigAction::Set(args) => {
                let s = ConfigSetArgs {
                    repo_root,
                    quiet: cli.quiet,
                    json: cli.json,
                    key: args.key,
                    value: args.value,
                };
                map_issue(config::run_set(s))
            }
        },
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
            IssueAction::Dep { action } => match action {
                DepAction::Add(args) => {
                    let rationale = if let Some(s) = args.rationale {
                        BodyInput::Inline(s)
                    } else if let Some(p) = args.rationale_file {
                        BodyInput::File(p)
                    } else if args.rationale_stdin {
                        BodyInput::Stdin
                    } else {
                        BodyInput::None
                    };
                    let add_args = DepAddArgs {
                        repo_root,
                        actor,
                        quiet: cli.quiet,
                        json: cli.json,
                        from_id: args.from,
                        to_id: args.to,
                        rationale,
                    };
                    map_issue(issue::dep::run_add(add_args))
                }
                DepAction::Remove(args) => {
                    let rm_args = DepRemoveArgs {
                        repo_root,
                        quiet: cli.quiet,
                        json: cli.json,
                        from_id: args.from,
                        to_id: args.to,
                    };
                    map_issue(issue::dep::run_remove(rm_args))
                }
            },
            IssueAction::Priority(args) => {
                let pri_args = PriorityArgs {
                    repo_root,
                    quiet: cli.quiet,
                    json: cli.json,
                    id: args.id,
                    priority: args.priority,
                };
                map_issue(issue::priority::run(pri_args))
            }
            IssueAction::Edit(args) => {
                let edit_args = EditArgs {
                    repo_root,
                    quiet: cli.quiet,
                    json: cli.json,
                    id: args.id,
                    title: args.title,
                    issue_type: args.issue_type,
                    epic: args.epic,
                };
                map_issue(issue::edit::run(edit_args))
            }
            IssueAction::Blocker { action } => match action {
                BlockerAction::Set(args) => {
                    let set_args = BlockerSetArgs {
                        repo_root,
                        actor,
                        quiet: cli.quiet,
                        json: cli.json,
                        id: args.id,
                        blocker: args.blocker,
                        comment: args.comment,
                    };
                    map_issue(issue::blocker::run_set(set_args))
                }
                BlockerAction::Clear(args) => {
                    let clear_args = BlockerClearArgs {
                        repo_root,
                        actor,
                        quiet: cli.quiet,
                        json: cli.json,
                        id: args.id,
                        comment: args.comment,
                    };
                    map_issue(issue::blocker::run_clear(clear_args))
                }
            },
            IssueAction::Close(args) => {
                let body = if let Some(s) = args.comment {
                    BodyInput::Inline(s)
                } else if let Some(p) = args.comment_file {
                    BodyInput::File(p)
                } else if args.comment_stdin {
                    BodyInput::Stdin
                } else {
                    BodyInput::None
                };
                let close_args = CloseArgs {
                    repo_root,
                    actor,
                    quiet: cli.quiet,
                    json: cli.json,
                    id: args.id,
                    dropped: args.dropped,
                    body,
                };
                map_issue(issue::close::run(close_args))
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
