use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgGroup, Args, Parser, Subcommand};

mod init;
mod issue;
mod storage;
mod time;

use issue::create::{BodyInput, CreateArgs, classify_exit_code};

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
                match issue::create::run(create_args) {
                    Ok(()) => Ok(0),
                    Err(e) => {
                        let code = classify_exit_code(&e);
                        eprintln!("error: {e:#}");
                        Ok(code)
                    }
                }
            }
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
