//! Mock implementations of Claude Code tools for the eval runner.
//! Per `docs/architecture/agent-eval.md`: Read is real (the agent
//! should be able to consult specs/source); everything else records
//! the call and returns a benign success stub.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::{Value, json};

use crate::eval::scenario::FixtureIssue;

/// Tool definition matching the Claude API's expected shape. Defined
/// here (not in an `anthropic` module) so the library is decoupled from
/// the dev-dep HTTP client.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// In-runner state shared across the tool-use loop. Each `dispatch`
/// call records the invocation here.
#[derive(Debug, Default)]
pub struct MockState {
    pub follow_up_queue: Mutex<Vec<String>>,
    pub fixture_issues: Vec<FixtureIssue>,
    pub repo_root: PathBuf,
}

impl MockState {
    pub fn new(fixture_issues: Vec<FixtureIssue>, follow_ups: Vec<String>, repo_root: PathBuf) -> Self {
        Self {
            follow_up_queue: Mutex::new(follow_ups),
            fixture_issues,
            repo_root,
        }
    }
}

/// Tool defs the eval runner advertises to the Claude API. We define a
/// small, fixed surface that matches Claude Code's built-in tools rather
/// than letting each agent's allowlist drive the list — the runner is
/// testing "what does the agent do given these tools," not "is the
/// allowlist enforced."
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "Read".into(),
            description: "Read a file from the repository.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }),
        },
        ToolDef {
            name: "Write".into(),
            description: "Write a file to the repository.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["file_path", "content"]
            }),
        },
        ToolDef {
            name: "Edit".into(),
            description: "Edit a file in the repository.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "old_string": {"type": "string"},
                    "new_string": {"type": "string"}
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        },
        ToolDef {
            name: "Bash".into(),
            description: "Run a shell command. Use for git and `dwarven` CLI invocations.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"},
                    "description": {"type": "string"}
                },
                "required": ["command"]
            }),
        },
        ToolDef {
            name: "AskUserQuestion".into(),
            description: "Ask the user a question to clarify intent.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "question": {"type": "string"},
                    "options": {"type": "array", "items": {"type": "object"}}
                },
                "required": ["question"]
            }),
        },
        ToolDef {
            name: "Agent".into(),
            description: "Dispatch a subagent.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "subagent_type": {"type": "string"},
                    "prompt": {"type": "string"}
                },
                "required": ["subagent_type", "prompt"]
            }),
        },
    ]
}

/// Dispatch a single tool call to its mock implementation. Returns
/// `(response_text, is_error)`. The eval runner records the invocation
/// in its own buffer; this function only synthesizes the tool_result.
pub fn dispatch(state: &MockState, name: &str, input: &Value) -> (String, bool) {
    match name {
        "Read" => mock_read(state, input),
        "Write" => mock_write(input),
        "Edit" => mock_edit(input),
        "Bash" => mock_bash(state, input),
        "AskUserQuestion" => mock_ask(state, input),
        "Agent" => mock_agent(input),
        other => (format!("unknown tool {other}; mocked as no-op"), false),
    }
}

fn mock_read(state: &MockState, input: &Value) -> (String, bool) {
    let path = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return ("missing file_path".into(), true),
    };
    let resolved = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        state.repo_root.join(path)
    };
    match std::fs::read_to_string(&resolved) {
        Ok(s) => (truncate_for_response(&s), false),
        Err(e) => (format!("(mock Read) failed: {e}"), true),
    }
}

fn truncate_for_response(s: &str) -> String {
    // Cap response size to keep API costs bounded and the agent focused.
    const MAX: usize = 8_000;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}\n...[truncated {} bytes]", &s[..MAX], s.len() - MAX)
    }
}

fn mock_write(input: &Value) -> (String, bool) {
    let path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    (format!("(mock Write) {path} written; no fs side effect"), false)
}

fn mock_edit(input: &Value) -> (String, bool) {
    let path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    (format!("(mock Edit) {path} edited; no fs side effect"), false)
}

fn mock_bash(state: &MockState, input: &Value) -> (String, bool) {
    let cmd = match input.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return ("missing command".into(), true),
    };
    // Pattern-match against `dwarven` read subcommands and return canned
    // JSON from the scenario's fixture_issues. Mutating dwarven commands
    // (transition, comment, close, etc.) just record + return success.
    let trimmed = cmd.trim();
    if let Some(id_str) = trimmed
        .split_whitespace()
        .skip_while(|t| t != &"view")
        .nth(1)
    {
        if let Ok(id) = id_str.parse::<u64>() {
            if let Some(issue) = state.fixture_issues.iter().find(|i| i.id == id) {
                return (serde_json::to_string_pretty(&fixture_to_json(issue)).unwrap(), false);
            }
            return (format!("(mock Bash) issue #{id} not in fixture"), true);
        }
    }
    if trimmed.contains("issue list") {
        let listed: Vec<_> = state
            .fixture_issues
            .iter()
            .map(fixture_to_json)
            .collect();
        return (serde_json::to_string_pretty(&listed).unwrap(), false);
    }
    if trimmed.starts_with("git ") {
        return (format!("(mock Bash git) recorded; no execution"), false);
    }
    (format!("(mock Bash) recorded; no execution: {cmd}"), false)
}

fn fixture_to_json(f: &FixtureIssue) -> Value {
    json!({
        "id": f.id,
        "title": f.title,
        "type": f.issue_type,
        "state": f.state,
        "priority": f.priority,
        "epic": f.epic,
        "blocked_by": f.blocked_by,
        "blocks": f.blocks,
        "body": f.body,
    })
}

fn mock_ask(state: &MockState, input: &Value) -> (String, bool) {
    let mut queue = state.follow_up_queue.lock().unwrap();
    if queue.is_empty() {
        let q = input.get("question").and_then(|v| v.as_str()).unwrap_or("?");
        return (
            format!("(mock AskUserQuestion) no answer queued for: {q}"),
            true,
        );
    }
    (queue.remove(0), false)
}

fn mock_agent(input: &Value) -> (String, bool) {
    let target = input
        .get("subagent_type")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    (
        format!("(mock Agent) subagent dispatch to '{target}' recorded; no execution"),
        false,
    )
}
