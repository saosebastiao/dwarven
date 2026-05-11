//! Claude Code render of the host-agnostic agent registry. The data
//! (descriptions, scope fences, allowlist patterns) lives in
//! `crate::adapter::registry`; this module only formats it into
//! Claude Code's `.claude/agents/<name>.md` frontmatter shape.

use crate::adapter::claude_code::commands::COMMAND_NAMES;
use crate::adapter::registry::{AgentDef, Mode};

// Re-export the registry surface so existing callers (and external
// crates / examples) can keep using `claude_code::agents::roster()`.
pub use crate::adapter::registry::roster;

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
