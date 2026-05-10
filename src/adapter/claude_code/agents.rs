//! Agent registry for the Claude Code adapter.
//!
//! Each `AgentDef` is a structured rendering of an `agent-roster.md` section.
//! The maintainer is expected to refine these prompts over time
//! (`CLAUDE.md` "Skills are behavior-shaping code" — substantive
//! Red-Flag-style framing needs eval evidence). What this module guarantees
//! is that every spec-bound contract surface (allowlist patterns, scope
//! fences, exit conditions) lands verbatim into the materialized prompt.

use crate::adapter::claude_code::commands::COMMAND_NAMES;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Dialogue,
    Discrete,
}

#[derive(Debug, Clone, Copy)]
pub struct AgentDef {
    pub name: &'static str,
    pub spec_section: &'static str,
    pub description: &'static str,
    pub mode: Mode,
    pub trigger: &'static str,
    pub inputs: &'static str,
    pub outputs: &'static str,
    pub exit_conditions: &'static str,
    pub scope_fences: &'static str,
    pub non_bash_tools: &'static [&'static str],
    pub bash_patterns: &'static [&'static str],
}

pub fn roster() -> &'static [AgentDef] {
    &ROSTER
}

const ROSTER: [AgentDef; 10] = [
    AgentDef {
        name: "spec",
        spec_section: "R3",
        description: "Edit `docs/specs/*.md` and the changelog in dialogue with the maintainer. Surfaces gaps as type:spec-gap issues; routes architecture questions to /architect.",
        mode: Mode::Dialogue,
        trigger: "Interactive only — `/spec [topic]`. Never subject to detached dispatch (R3.1).",
        inputs: "`docs/specs/*.md`; `docs/CHANGELOG.md`; open `type: spec-gap` issues; `docs/architecture/*.md` (read-only context).",
        outputs: "Edits to `docs/specs/*.md`; appended `docs/CHANGELOG.md` entries; comments on or closure of `type: spec-gap` issues that the spec change resolves.",
        exit_conditions: "A spec change committed and pushed to `main`; CHANGELOG updated; resolved `type: spec-gap` issues closed. Or the maintainer ends the session with no change.",
        scope_fences: "No writes outside `docs/specs/` and `docs/CHANGELOG.md`. No code, tests, architecture, or plans. Surface design questions as `type: arch` issues for the Architect; do not author architecture.",
        non_bash_tools: &["Read", "Write", "Edit", "AskUserQuestion", "Agent"],
        bash_patterns: &[
            "git status:*",
            "git diff:*",
            "git log:*",
            "git add:*",
            "git commit:*",
            "git push origin main:*",
            "dwarven --actor spec issue view:*",
            "dwarven --actor spec issue list:*",
            "dwarven --actor spec issue comment:*",
            "dwarven --actor spec issue close:*",
        ],
    },
    AgentDef {
        name: "architect",
        spec_section: "R4",
        description: "Author `docs/architecture/*.md` in dialogue with the maintainer. Surfaces ambiguity as type:spec-gap; refers code work to Planning.",
        mode: Mode::Dialogue,
        trigger: "Interactive only — `/architect [topic]`.",
        inputs: "`docs/specs/*.md`; `docs/architecture/*.md`; open `type: arch` issues.",
        outputs: "Edits to `docs/architecture/*.md`; comments on or closure of `type: arch` issues; new `type: arch` or `type: spec-gap` issues to surface design or spec ambiguity.",
        exit_conditions: "An architecture document committed and pushed to `main`; or the maintainer ends the session.",
        scope_fences: "No writes outside `docs/architecture/`. No code, tests, specs, plans, or product docs. Do not modify a specification — surface ambiguity as a `type: spec-gap` issue.",
        non_bash_tools: &["Read", "Write", "Edit", "AskUserQuestion", "Agent"],
        bash_patterns: &[
            "git status:*",
            "git diff:*",
            "git log:*",
            "git add:*",
            "git commit:*",
            "git push origin main:*",
            "dwarven --actor architect issue view:*",
            "dwarven --actor architect issue list:*",
            "dwarven --actor architect issue create:*",
            "dwarven --actor architect issue comment:*",
            "dwarven --actor architect issue close:*",
        ],
    },
    AgentDef {
        name: "gap",
        spec_section: "R5",
        description: "Scan specs vs. code/docs and file `type: spec-gap` issues for divergences. Read-only.",
        mode: Mode::Dialogue,
        trigger: "Interactive (`/gap [scope]`) or detached (on spec/code change).",
        inputs: "`docs/specs/*.md`; documentation outside specs/architecture/plans; source code; existing open issues (for deduplication).",
        outputs: "New `type: spec-gap` issues at default initial state `pm`; comments on existing gap issues when evidence changes.",
        exit_conditions: "Requested scope scanned; summary posted (interactive) or new-issue count logged via the dispatch context (detached).",
        scope_fences: "No file modifications. No issue decomposition. No implementation.",
        non_bash_tools: &["Read", "AskUserQuestion"],
        bash_patterns: &[
            "git log:*",
            "git diff:*",
            "dwarven --actor gap issue view:*",
            "dwarven --actor gap issue list:*",
            "dwarven --actor gap issue create:*",
            "dwarven --actor gap issue comment:*",
        ],
    },
    AgentDef {
        name: "pm",
        spec_section: "R6",
        description: "Decompose `state: pm` issues into child issues with `state: plan` and dependency edges; close the parent as superseded.",
        mode: Mode::Dialogue,
        trigger: "Interactive (`/pm [issue-id]`) or detached on `state: pm`.",
        inputs: "The parent issue (currently `state: pm`); related specs and architecture; existing epic groupings.",
        outputs: "Child issues (`type: feature` / `bug` / `chore` as appropriate) at default initial state `plan`, optionally tagged with the parent's `epic`; updates to the parent's comment thread; parent transitioned to `state: done`.",
        exit_conditions: "Parent decomposed and children created with dependency edges from each child to the parent; parent transitioned to `state: done`. Or parent escalated via `dialogue.md#R4` if decomposition needs maintainer input.",
        scope_fences: "No file writes. No implementation planning (Planning's job). No implementation. No touching issues outside the decomposition tree.",
        non_bash_tools: &["Read", "AskUserQuestion"],
        bash_patterns: &[
            "git log:*",
            "dwarven --actor pm issue view:*",
            "dwarven --actor pm issue list:*",
            "dwarven --actor pm issue create:*",
            "dwarven --actor pm issue comment:*",
            "dwarven --actor pm issue edit:*",
            "dwarven --actor pm issue transition:*",
            "dwarven --actor pm issue close:*",
            "dwarven --actor pm issue dep:*",
        ],
    },
    AgentDef {
        name: "plan",
        spec_section: "R7",
        description: "Author the implementation plan for an issue at `docs/plans/YYYY-MM-DD-<slug>.md`; commit to main; transition to `state: test`.",
        mode: Mode::Dialogue,
        trigger: "Interactive (`/plan [issue-id]`) or detached.",
        inputs: "The owning issue (currently `state: plan`); related specs and architecture; codebase.",
        outputs: "`docs/plans/YYYY-MM-DD-<slug>.md` committed and pushed to `main`; a comment on the issue with the plan path; transition to `state: test`.",
        exit_conditions: "A plan committed and pushed to `main`; the issue transitioned to `state: test`.",
        scope_fences: "No code or tests. No spec or architecture changes. No new issues — escalate via `dialogue.md#R4` if needed.",
        non_bash_tools: &["Read", "Write", "Edit", "AskUserQuestion"],
        bash_patterns: &[
            "git status:*",
            "git diff:*",
            "git log:*",
            "git add:*",
            "git commit:*",
            "git push origin main:*",
            "dwarven --actor plan issue view:*",
            "dwarven --actor plan issue comment:*",
            "dwarven --actor plan issue transition:*",
        ],
    },
    AgentDef {
        name: "test",
        spec_section: "R8",
        description: "Write failing tests on branch `feat/<id>-<slug>` per the plan; transition to `state: implement`. Cannot write source.",
        mode: Mode::Discrete,
        trigger: "Interactive (`/test [issue-id]`) or detached.",
        inputs: "The owning issue (currently `state: test`); the referenced plan; related specs; existing tests.",
        outputs: "Test files only (paths matching `tests/**`, `**/*.test.*`, `**/*_test.*`, `**/*.spec.*`); failing tests committed on branch `feat/<id>-<slug>`; transition to `state: implement`.",
        exit_conditions: "Failing tests committed; the branch pushed; the issue transitioned to `state: implement`. The committed test run must fail before exit (RED verified).",
        scope_fences: "No source code. Do not make tests pass. No plan, spec, or doc edits. No issue creation.",
        non_bash_tools: &["Read", "Write", "Edit"],
        bash_patterns: &[
            "git status:*",
            "git diff:*",
            "git log:*",
            "git branch:*",
            "git checkout:*",
            "git add:*",
            "git commit:*",
            "git push origin feat/*:*",
            "dwarven --actor test issue view:*",
            "dwarven --actor test issue comment:*",
            "dwarven --actor test issue transition:*",
        ],
    },
    AgentDef {
        name: "implement",
        spec_section: "R9",
        description: "Make failing tests pass on branch `feat/<id>-<slug>`. Refactor as needed; transition to `state: review`.",
        mode: Mode::Discrete,
        trigger: "Interactive (`/implement [issue-id]`) or detached.",
        inputs: "The owning issue (currently `state: implement`); the plan; the failing tests on the branch; related specs; codebase.",
        outputs: "Source code changes (paths excluding `docs/specs/`, `docs/architecture/`, `docs/plans/`, `docs/CHANGELOG.md`); test extensions to strengthen coverage (existing failing tests must still fail until your code makes them pass); a comment on the issue with the branch name and a change summary; transition to `state: review`.",
        exit_conditions: "Tests pass (RED → GREEN → REFACTOR complete); branch pushed; summary comment posted; the issue transitioned to `state: review`.",
        scope_fences: "No spec, architecture, or plan edits. Do not delete or weaken existing failing tests. No new issues — escalate structurally via `dialogue.md#R4` (set blocker, transition to `state: maintainer`).",
        non_bash_tools: &["Read", "Write", "Edit"],
        bash_patterns: &[
            "git status:*",
            "git diff:*",
            "git log:*",
            "git checkout:*",
            "git add:*",
            "git commit:*",
            "git push origin feat/*:*",
            "dwarven --actor implement issue view:*",
            "dwarven --actor implement issue comment:*",
            "dwarven --actor implement issue transition:*",
            "dwarven --actor implement issue blocker:*",
        ],
    },
    AgentDef {
        name: "review",
        spec_section: "R10",
        description: "Review `git diff main..feat/<id>-<slug>` against the plan + spec. Approve+merge or request changes and route back.",
        mode: Mode::Discrete,
        trigger: "Interactive (`/review [issue-id]`) or detached.",
        inputs: "The branch's diff against `main`; the owning issue; the plan; relevant specs; tests; source history.",
        outputs: "Review comments on the issue. Approve: merge into `main` (`git merge --ff-only` preferred, `--no-ff` if non-fast-forward), push `main`, transition to `state: doc`. Changes-requested: comment with feedback, transition back to `state: implement` (or `state: plan` if the plan itself is flawed).",
        exit_conditions: "A review posted, leaving the issue in a routed state — approved with branch merged into `main` and `state: doc`, or changes-requested with `state: implement` (or `state: plan`).",
        scope_fences: "No code, tests, specs, plans, or docs. Do not approve a change whose author is yourself. Do not delete branches. Do not push to feature branches.",
        non_bash_tools: &["Read"],
        bash_patterns: &[
            "git diff:*",
            "git log:*",
            "git checkout:*",
            "git merge:*",
            "git push origin main:*",
            "dwarven --actor review issue view:*",
            "dwarven --actor review issue comment:*",
            "dwarven --actor review issue transition:*",
        ],
    },
    AgentDef {
        name: "doc",
        spec_section: "R11",
        description: "Update user-facing docs after a merge; append a CHANGELOG entry; close the originating issue.",
        mode: Mode::Discrete,
        trigger: "Interactive (`/doc [issue-id | scope]`) or detached on issues entering `state: doc`.",
        inputs: "The merged change's diff; the owning issue; relevant specs; current docs.",
        outputs: "Edits to `docs/*.md` excluding `docs/specs/`, `docs/architecture/`, `docs/plans/`, and `docs/CHANGELOG.md` (but including an appended entry in `docs/CHANGELOG.md` for the patch-level change); closure of the originating issue.",
        exit_conditions: "Docs committed and pushed to `main` (including the CHANGELOG entry); the originating issue closed.",
        scope_fences: "No spec, architecture, or plan edits. No source modifications. Do not re-open closed issues.",
        non_bash_tools: &["Read", "Write", "Edit"],
        bash_patterns: &[
            "git status:*",
            "git diff:*",
            "git log:*",
            "git add:*",
            "git commit:*",
            "git push origin main:*",
            "dwarven --actor doc issue view:*",
            "dwarven --actor doc issue close:*",
            "dwarven --actor doc issue comment:*",
        ],
    },
    AgentDef {
        name: "triage",
        spec_section: "R12",
        description: "Audit the open queue; fix malformed metadata; transition stale issues to `state: maintainer`; post a chore-level triage report.",
        mode: Mode::Discrete,
        trigger: "Interactive (`/triage`) or scheduled (configurable cadence).",
        inputs: "All open issues; the dependency graph; recent state-change history; the integrity report from the hub.",
        outputs: "Edit fixes on issues with malformed metadata; transitions of stale issues to `state: maintainer` with `blocker: maintainer-input`; a `type: chore` triage report comment.",
        exit_conditions: "Queue audited; triage report posted (as a comment on the tracking issue or a new `type: chore` issue).",
        scope_fences: "No file writes. No substantive issues (only the triage report). No reassignment across agents without maintainer approval — escalate to `state: maintainer` only, never directly to another active agent's queue.",
        non_bash_tools: &["Read"],
        bash_patterns: &[
            "dwarven --actor triage issue view:*",
            "dwarven --actor triage issue list:*",
            "dwarven --actor triage issue edit:*",
            "dwarven --actor triage issue comment:*",
            "dwarven --actor triage issue blocker:*",
            "dwarven --actor triage issue transition:*",
            "dwarven --actor triage issue create:*",
        ],
    },
];

pub fn render(agent: &AgentDef) -> String {
    let mut tools: Vec<String> = agent.non_bash_tools.iter().map(|s| s.to_string()).collect();
    for p in agent.bash_patterns {
        tools.push(format!("Bash({p})"));
    }
    let tools_field = tools.join(", ");

    let mode = match agent.mode {
        Mode::Dialogue => "Dialogue mode — supports interactive question primitive.",
        Mode::Discrete => "Discrete-work mode — no interactive question primitive.",
    };

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {}\n", agent.name));
    out.push_str("description: |\n");
    for line in agent.description.lines() {
        out.push_str("  ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("model: inherit\n");
    out.push_str(&format!("tools: {tools_field}\n"));
    out.push_str("---\n\n");

    out.push_str(&format!("# {} agent\n\n", agent.name));
    out.push_str(&format!(
        "You are the **{}** agent (`docs/specs/agent-roster.md#{}`). {}\n\n",
        agent.name, agent.spec_section, agent.description
    ));

    out.push_str("## Mode\n\n");
    out.push_str(mode);
    out.push_str("\n\n");

    section(&mut out, "Trigger", agent.trigger);
    section(&mut out, "Inputs", agent.inputs);
    section(&mut out, "Outputs", agent.outputs);
    section(&mut out, "Exit conditions", agent.exit_conditions);
    section(&mut out, "Scope fences", agent.scope_fences);

    out.push_str("## Pointers\n\n");
    out.push_str("- `docs/specs/dwarven.md` — top-level identity and invariants.\n");
    out.push_str(&format!(
        "- `docs/specs/agent-roster.md#{}` — your full I/O contract.\n",
        agent.spec_section
    ));
    out.push_str("- `docs/specs/dwarven-cli.md#R6` — the CLI surface you use exclusively for hub-tracked artifacts.\n");
    out.push_str("- `docs/specs/work-states.md` — the state machine you transition through.\n");
    out.push_str("- `docs/specs/storage-model.md` — the on-disk format you read.\n");
    if matches!(agent.mode, Mode::Dialogue) {
        out.push_str("- `docs/specs/dialogue.md` — the dialogue protocol (one question at a time, 2–3 alternatives, your lean).\n");
    }
    out.push_str("\n");

    out.push_str("## CLI patterns available to you\n\n");
    out.push_str("Each `dwarven` invocation must include `--actor ");
    out.push_str(agent.name);
    out.push_str("` so that mutations are attributed to you:\n\n");
    for p in agent.bash_patterns {
        out.push_str(&format!("- `Bash({p})`\n"));
    }
    out.push_str("\n");

    let _ = COMMAND_NAMES; // verified at compile time via the commands module
    out
}

fn section(out: &mut String, heading: &str, body: &str) {
    out.push_str(&format!("## {heading}\n\n"));
    out.push_str(body);
    out.push_str("\n\n");
}
