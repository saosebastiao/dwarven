//! `cargo run --example eval-runner` — agent prompt eval runner.
//! Spec: `docs/architecture/agent-eval.md`. Plan:
//! `docs/plans/2026-05-10-issue-10-eval-runner.md`.
//!
//! The Anthropic HTTP client and the run loop live here because
//! `reqwest` is a dev-dependency — accessible from examples and tests,
//! but not from the production library. Pure parts (scenario YAML,
//! tool-call matcher, mock tools) live in `src/eval/`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use dwarven::adapter::claude_code::agents;
use dwarven::eval::matcher::{
    ToolInvocation, forbidden_call_violations, required_call_satisfied,
};
use dwarven::eval::mock_tools::{MockState, dispatch, tool_definitions};
use dwarven::eval::scenario::{self, Scenario, ToolPattern};

const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const MAX_TURNS: usize = 12;

#[derive(Parser, Debug)]
#[command(
    name = "eval-runner",
    about = "Agent prompt eval runner (docs/architecture/agent-eval.md)"
)]
struct Args {
    /// Restrict to scenarios under `evals/<agent>/`.
    #[arg(long)]
    agent: Option<String>,

    /// Run only the scenario at this path.
    #[arg(long)]
    scenario: Option<PathBuf>,

    /// Override the Claude model (default: claude-sonnet-4-6 or
    /// $DWARVEN_EVAL_MODEL).
    #[arg(long)]
    model: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("eval-runner: {e:#}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<u8> {
    let args = Args::parse();

    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        anyhow!(
            "ANTHROPIC_API_KEY is required. Set it before running evals so a CI \
             misconfiguration can't silently pass evals that never ran."
        )
    })?;
    let model = args
        .model
        .clone()
        .or_else(|| std::env::var("DWARVEN_EVAL_MODEL").ok())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let scenarios = collect_scenarios(&args)?;
    if scenarios.is_empty() {
        eprintln!("no scenarios matched");
        return Ok(0);
    }

    let repo_root = std::env::current_dir().context("getting current dir")?;
    let client = AnthropicClient::new(api_key, model.clone());
    let mut results: Vec<ScenarioResult> = Vec::with_capacity(scenarios.len());

    for path in scenarios {
        let scenario = scenario::load(&path)
            .with_context(|| format!("loading {}", path.display()))?;
        eprint!("running {}... ", path.display());
        let result =
            run_scenario(&client, scenario, path.clone(), repo_root.clone()).await;
        match result {
            Ok(r) => {
                eprintln!(
                    "{} ({:.1}s, ${:.4})",
                    if r.passed { "pass" } else { "FAIL" },
                    r.duration_secs,
                    r.cost_usd()
                );
                for v in &r.violations {
                    eprintln!("    {v}");
                }
                results.push(r);
            }
            Err(e) => {
                eprintln!("ERROR: {e:#}");
                results.push(ScenarioResult::error(path, e));
            }
        }
    }

    let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();
    let total_cost: f64 = results.iter().map(|r| r.cost_usd()).sum();
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    eprintln!(
        "{}/{} passed in {:.1}s, total ${:.4}",
        passed, total, total_duration, total_cost
    );

    Ok(if passed == total { 0 } else { 1 })
}

fn collect_scenarios(args: &Args) -> Result<Vec<PathBuf>> {
    if let Some(p) = &args.scenario {
        return Ok(vec![p.clone()]);
    }
    let root = match &args.agent {
        Some(a) => Path::new("evals").join(a),
        None => Path::new("evals").to_path_buf(),
    };
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    collect_yaml(&root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_yaml(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_yaml(&path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "yaml" || e == "yml")
        {
            out.push(path);
        }
    }
    Ok(())
}

// ---- Anthropic API client (dev-dep reqwest) ----

#[derive(Debug, Clone, Serialize)]
struct Message {
    role: String,
    content: Vec<MessageContent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum MessageContent {
    Text {
        #[serde(rename = "type")]
        kind: String,
        text: String,
    },
    ToolUse {
        #[serde(rename = "type")]
        kind: String,
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        #[serde(rename = "type")]
        kind: String,
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl MessageContent {
    fn text(s: impl Into<String>) -> Self {
        Self::Text {
            kind: "text".into(),
            text: s.into(),
        }
    }

    fn tool_use(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self::ToolUse {
            kind: "tool_use".into(),
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self::ToolResult {
            kind: "tool_result".into(),
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: if is_error { Some(true) } else { None },
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiResponse {
    content: Vec<ResponseBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ResponseBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

impl Usage {
    fn cost_usd(&self) -> f64 {
        (self.input_tokens as f64) * 3.0 / 1_000_000.0
            + (self.output_tokens as f64) * 15.0 / 1_000_000.0
    }
}

struct AnthropicClient {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }

    async fn send(
        &self,
        system: &str,
        messages: &[Message],
        tools: &Value,
    ) -> Result<ApiResponse> {
        let body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": messages,
            "tools": tools,
        });
        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("sending Claude API request")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Claude API {status}: {body}"));
        }
        resp.json::<ApiResponse>()
            .await
            .context("parsing Claude API response")
    }
}

// ---- Run loop ----

#[derive(Debug)]
#[allow(dead_code)] // scenario_path + agent surface in future report extensions
struct ScenarioResult {
    scenario_path: PathBuf,
    agent: String,
    passed: bool,
    violations: Vec<String>,
    usage: Usage,
    duration_secs: f64,
}

impl ScenarioResult {
    fn cost_usd(&self) -> f64 {
        self.usage.cost_usd()
    }

    fn error(path: PathBuf, e: anyhow::Error) -> Self {
        Self {
            scenario_path: path,
            agent: "<unknown>".into(),
            passed: false,
            violations: vec![format!("runner error: {e:#}")],
            usage: Usage::default(),
            duration_secs: 0.0,
        }
    }
}

async fn run_scenario(
    client: &AnthropicClient,
    scenario: Scenario,
    scenario_path: PathBuf,
    repo_root: PathBuf,
) -> Result<ScenarioResult> {
    let start = Instant::now();

    let agent_def = agents::roster()
        .iter()
        .find(|a| a.name == scenario.agent)
        .ok_or_else(|| anyhow!("unknown agent '{}' in scenario", scenario.agent))?;
    let system_prompt = agents::render(agent_def);
    // Build tools as a JSON array directly so we can hand it to the API.
    let tools_array: Value = serde_json::to_value(tool_definitions())?;
    let state = MockState::new(
        scenario.context.fixture_issues.clone(),
        scenario.follow_up_answers.clone(),
        repo_root,
    );

    let mut messages: Vec<Message> = vec![Message {
        role: "user".into(),
        content: vec![MessageContent::text(&scenario.user_message)],
    }];
    let mut invocations: Vec<ToolInvocation> = Vec::new();
    let mut total_usage = Usage::default();
    let mut final_text = String::new();

    for turn in 1..=MAX_TURNS {
        let resp = client
            .send(&system_prompt, &messages, &tools_array)
            .await
            .with_context(|| format!("API call on turn {turn}"))?;
        total_usage.input_tokens += resp.usage.input_tokens;
        total_usage.output_tokens += resp.usage.output_tokens;

        let mut tool_uses: Vec<(String, String, Value)> = Vec::new();
        let mut text_chunks: Vec<String> = Vec::new();
        for block in &resp.content {
            match block {
                ResponseBlock::Text { text } => text_chunks.push(text.clone()),
                ResponseBlock::ToolUse { id, name, input } => {
                    invocations.push(ToolInvocation {
                        name: name.clone(),
                        input: input.clone(),
                        turn,
                    });
                    tool_uses.push((id.clone(), name.clone(), input.clone()));
                }
            }
        }
        final_text = text_chunks.join("\n");

        if tool_uses.is_empty() {
            break;
        }

        let assistant_content: Vec<MessageContent> = resp
            .content
            .iter()
            .map(|b| match b {
                ResponseBlock::Text { text } => MessageContent::text(text),
                ResponseBlock::ToolUse { id, name, input } => {
                    MessageContent::tool_use(id, name, input.clone())
                }
            })
            .collect();
        messages.push(Message {
            role: "assistant".into(),
            content: assistant_content,
        });

        let mut tool_results: Vec<MessageContent> = Vec::new();
        for (id, _name, input) in &tool_uses {
            let name = invocations.last().map(|i| i.name.as_str()).unwrap_or("");
            let _ = (id, name, input); // for clarity
            // re-locate the right invocation by id
            let tool_name = tool_uses
                .iter()
                .find(|t| t.0 == *id)
                .map(|t| t.1.as_str())
                .unwrap_or("");
            let tool_input = tool_uses
                .iter()
                .find(|t| t.0 == *id)
                .map(|t| &t.2)
                .unwrap_or(input);
            let (response, is_error) = dispatch(&state, tool_name, tool_input);
            tool_results.push(MessageContent::tool_result(id, response, is_error));
        }
        messages.push(Message {
            role: "user".into(),
            content: tool_results,
        });
    }

    let mut violations: Vec<String> = Vec::new();
    for pat in &scenario.required_tool_calls {
        if !required_call_satisfied(pat, &invocations) {
            violations.push(format!(
                "required tool call not invoked: {}",
                describe_pattern(pat)
            ));
        }
    }
    for pat in &scenario.forbidden_tool_calls {
        let offending = forbidden_call_violations(pat, &invocations);
        for idx in offending {
            let inv = &invocations[idx];
            violations.push(format!(
                "forbidden tool call observed: {} at turn {} input={}",
                describe_pattern(pat),
                inv.turn,
                inv.input
            ));
        }
    }

    if let Some(judge) = &scenario.response_judge {
        if judge.required {
            match run_judge(client, &judge.prompt, &final_text).await {
                Ok((passed, reason, usage)) => {
                    total_usage.input_tokens += usage.input_tokens;
                    total_usage.output_tokens += usage.output_tokens;
                    if !passed {
                        violations.push(format!("response_judge FAIL: {reason}"));
                    }
                }
                Err(e) => violations.push(format!("response_judge call errored: {e:#}")),
            }
        }
    }

    Ok(ScenarioResult {
        scenario_path,
        agent: scenario.agent,
        passed: violations.is_empty(),
        violations,
        usage: total_usage,
        duration_secs: start.elapsed().as_secs_f64(),
    })
}

fn describe_pattern(p: &ToolPattern) -> String {
    let mut parts = vec![p.pattern.clone()];
    if let Some(g) = &p.path_glob {
        parts.push(format!("path_glob={g}"));
    }
    if let Some(c) = &p.contains {
        parts.push(format!("contains={c}"));
    }
    if let Some(t) = p.after_turn {
        parts.push(format!("after_turn={t}"));
    }
    if let Some(t) = p.before_turn {
        parts.push(format!("before_turn={t}"));
    }
    parts.join(" ")
}

async fn run_judge(
    client: &AnthropicClient,
    rubric: &str,
    response: &str,
) -> Result<(bool, String, Usage)> {
    let prompt = format!(
        "{rubric}\n\nAgent response:\n{response}\n\nReply with exactly \"PASS\" or \"FAIL\" followed by a brief reason on the same line."
    );
    let messages = vec![Message {
        role: "user".into(),
        content: vec![MessageContent::text(&prompt)],
    }];
    let empty_tools = json!([]);
    let api_resp = client.send("", &messages, &empty_tools).await?;
    let text = api_resp
        .content
        .iter()
        .find_map(|b| match b {
            ResponseBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    let passed = lower.starts_with("pass");
    let reason = trimmed
        .split_once(' ')
        .map(|(_, r)| r.to_string())
        .unwrap_or_default();
    Ok((passed, reason, api_resp.usage))
}
