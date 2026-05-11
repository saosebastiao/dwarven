---
issue: 10
date: 2026-05-10
---

# Plan: eval runner implementation

**Issue:** #10
**Spec:** `docs/architecture/agent-eval.md` (issue #7)

## Goal

Ship the runner that consumes the YAML scenario format spec'd in #7. Invoked via `cargo run --example eval-runner -- [--agent N] [--scenario PATH]`. Refuses to start without `ANTHROPIC_API_KEY`. Loads materialized agent prompts from `crate::adapter::claude_code::agents` (single source of truth with the production adapter), drives the Claude API tool-use loop against mock implementations, asserts on tool-call patterns + optional judge, reports per-scenario pass/fail.

## Library-crate refactor

`examples/` can only consume library targets. The crate currently exposes only a `[[bin]]`. Minimal fix: add `src/lib.rs` that re-declares the crate's modules as `pub mod` declarations. Modules compile under both bin and lib targets. Existing code paths unchanged; existing tests under `tests/` keep working (they invoke the compiled bin via `assert_cmd`).

## Dependencies

Dev-deps only (the eval runner doesn't ship in the production binary):

- `reqwest = { version = "0.12", features = ["json", "rustls-tls"] }` — Claude API HTTP client.
- `globset = "0.4"` — glob matcher for `path_glob` patterns.

`serde_yaml`, `serde`, `serde_json`, `tokio` already in tree.

## Module layout

```
examples/eval_runner.rs       # main entry, CLI arg parsing
src/eval/
  mod.rs                      # re-exports
  scenario.rs                 # YAML deserialization
  matcher.rs                  # ToolPattern + matching logic
  runner.rs                   # API loop driver
  mock_tools.rs               # mocks for Edit/Write/Bash/AskUserQuestion
  anthropic.rs                # thin Claude API client
```

## Scenario types (`src/eval/scenario.rs`)

```rust
pub struct Scenario {
    pub agent: String,
    pub context: Context,
    pub user_message: String,
    pub follow_up_answers: Vec<String>,
    pub required_tool_calls: Vec<ToolPattern>,
    pub forbidden_tool_calls: Vec<ToolPattern>,
    pub response_judge: Option<JudgeRubric>,
}
pub struct ToolPattern {
    pub pattern: String,
    pub path_glob: Option<String>,
    pub contains: Option<String>,
    pub before_turn: Option<usize>,
    pub after_turn: Option<usize>,
}
```

## Matcher (`src/eval/matcher.rs`)

Pure function operating on captured `ToolInvocation { name, input: Value, turn: usize }`:

- `pattern_matches(pattern, inv) -> bool`
- `required_call_satisfied(pattern, invocations) -> bool`
- `forbidden_call_violations(pattern, invocations) -> Vec<usize>` (offending invocation indices)

Unit tests cover name match, path_glob, contains, turn windows.

## Anthropic API client (`src/eval/anthropic.rs`)

Minimal: just `POST /v1/messages`. No streaming. Maps Claude API content blocks to a local enum (`Text(String)` / `ToolUse { id, name, input }`).

Default model: `claude-sonnet-4-6` (overridable via `--model` or `DWARVEN_EVAL_MODEL`).

## Runner (`src/eval/runner.rs`)

Per scenario:

1. Resolve agent system prompt via `agents::roster()` + `agents::render()`.
2. Build the Claude API `tools` list with hand-written schemas for Edit / Write / Read / Bash / AskUserQuestion / Agent.
3. Init messages: `[{role: user, content: scenario.user_message}]`.
4. Loop (max 12 turns):
   - Call client.
   - For each `ToolUse` in response: record `ToolInvocation`, dispatch to mock, build `tool_result` block.
   - If no `ToolUse`: done.
   - Else: append assistant + user(tool_result) messages, continue.
5. Assertions: required + forbidden patterns.
6. If `response_judge.required`: judge call (rubric + agent's final text), parse first PASS/FAIL token.
7. Return `ScenarioResult { passed, violations, cost_estimate, duration }`.

## Mock tools (`src/eval/mock_tools.rs`)

- **`Read`**: real fs read (the agent should consult specs/source).
- **`Edit` / `Write`**: record + success stub. Never touch disk.
- **`Bash`**: parse the command; if `dwarven --actor <name> issue view/list`, return canned JSON from `fixture_issues`; if `git`, return a generic success stub; otherwise record + stub.
- **`AskUserQuestion`**: dequeue from `follow_up_answers`; if empty, return a no-answer error.
- **`Agent`**: record + stub; no recursion.

## CLI (`examples/eval_runner.rs`)

```
cargo run --example eval-runner -- [--agent <N>] [--scenario <PATH>] [--model <ID>]
```

Refuses without `ANTHROPIC_API_KEY`. Default model `claude-sonnet-4-6`. Output per the spec doc. Exit code 0 on all-pass, 1 on any failure.

## Tests

Unit tests in `src/eval/matcher.rs#tests` (pattern matching) and `src/eval/scenario.rs#tests` (YAML round-trip with the two reference scenarios).

No automated E2E hitting the API. Manual smoke: maintainer runs the runner with their API key against the two reference scenarios.

## Branch

`feat/10-eval-runner`. Commits scoped to lib refactor, scenario+matcher, anthropic+runner, examples/cli.
