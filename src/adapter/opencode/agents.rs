//! opencode render of the host-agnostic agent registry. Produces
//! `.opencode/agents/<name>.md` with opencode's frontmatter shape:
//! `description`, `mode: subagent`, `model: inherit`, and a
//! `permission` block per `host-adapter.md#R4.6` + `#R4.7`.

use crate::adapter::registry::{AgentDef, Mode};

pub fn render(agent: &AgentDef) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("description: {}\n", oneline(agent.description)));
    out.push_str("mode: subagent\n");
    out.push_str("model: inherit\n");
    render_permission_block(&mut out, agent);
    out.push_str("---\n\n");

    // Prompt body — identical to the Claude Code adapter modulo
    // frontmatter, so the agent behaves the same regardless of host.
    out.push_str(&format!("# {} agent\n\n", agent.name));
    out.push_str(&format!(
        "You are the **{}** agent (`docs/specs/agent-roster.md#{}`). {}\n\n",
        agent.name, agent.spec_section, agent.description
    ));

    let mode = match agent.mode {
        Mode::Dialogue => "Dialogue mode — supports the `question` primitive (free-text under opencode; see `host-adapter.md#R4.4.2`).",
        Mode::Discrete => "Discrete-work mode — no `question` primitive.",
    };
    out.push_str("## Mode\n\n");
    out.push_str(mode);
    out.push_str("\n\n");

    section(&mut out, "Trigger", agent.trigger);
    section(&mut out, "Inputs", agent.inputs);
    section(&mut out, "Outputs", agent.outputs);
    section(&mut out, "Exit conditions", agent.exit_conditions);
    section(&mut out, "Scope fences", agent.scope_fences);
    if !agent.red_flags.is_empty() {
        section(&mut out, "Red flags", agent.red_flags);
    }
    if !agent.verification.is_empty() {
        section(&mut out, "Verification before exit", agent.verification);
    }

    out.push_str("## Pointers\n\n");
    out.push_str("- `docs/specs/dwarven.md` — top-level identity and invariants.\n");
    out.push_str(&format!(
        "- `docs/specs/agent-roster.md#{}` — your full I/O contract.\n",
        agent.spec_section
    ));
    out.push_str("- `docs/specs/dwarven-cli.md#R6` — the CLI surface you use exclusively for hub-tracked artifacts.\n");
    out.push_str("- `docs/specs/work-states.md` — the state machine you transition through.\n");
    out.push_str("- `docs/specs/storage-model.md` — the on-disk format you read.\n");
    out.push_str("- `docs/specs/host-adapter.md#R4` — opencode adapter contract.\n");
    if matches!(agent.mode, Mode::Dialogue) {
        out.push_str("- `docs/specs/dialogue.md` — the dialogue protocol. Note: under opencode the `question` primitive is free-text; render alternatives as Markdown in your response.\n");
    }
    out.push_str("\n");

    out.push_str("## CLI patterns available to you\n\n");
    out.push_str("Each `dwarven` invocation must include `--actor ");
    out.push_str(agent.name);
    out.push_str("` so that mutations are attributed to you. The permission block above enforces this structurally:\n\n");
    for p in agent.bash_patterns {
        out.push_str(&format!("- `{}` (allowed)\n", strip_claude_pattern(p)));
    }
    out.push_str("\n");

    out
}

/// Render the opencode permission block for a single agent's frontmatter.
fn render_permission_block(out: &mut String, agent: &AgentDef) {
    out.push_str("permission:\n");
    out.push_str("  read: allow\n");

    // edit / write scope per non_bash_tools. If Write or Edit is in
    // non_bash_tools, allow on the agent's writable paths; deny otherwise.
    // The writable-paths set is inferred from the agent's scope_fences;
    // for v3 we set a conservative `*: deny` and let the system prompt
    // describe the actual paths the agent is supposed to touch. Future
    // refinement: structure the AgentDef to carry writable_paths
    // explicitly so opencode's path-glob permissions get full
    // mechanism-enforced scope-fencing.
    let allows_write = agent
        .non_bash_tools
        .iter()
        .any(|t| matches!(*t, "Write" | "Edit"));
    if allows_write {
        out.push_str("  edit: allow\n");
        out.push_str("  write: allow\n");
    } else {
        out.push_str("  edit: deny\n");
        out.push_str("  write: deny\n");
    }

    // task (subagent dispatch) — restricted per `agent-roster.md#R13.5`.
    let allows_task = agent.non_bash_tools.iter().any(|t| *t == "Agent");
    if allows_task {
        out.push_str("  task: allow\n");
    } else {
        out.push_str("  task: deny\n");
    }

    // question — dialogue agents only.
    match agent.mode {
        Mode::Dialogue => out.push_str("  question: allow\n"),
        Mode::Discrete => out.push_str("  question: deny\n"),
    }

    // bash — per-pattern allow + trailing `*: deny`.
    out.push_str("  bash:\n");
    for p in agent.bash_patterns {
        out.push_str(&format!("    \"{}\": allow\n", to_opencode_pattern(p)));
    }
    out.push_str("    \"*\": deny\n");
}

fn section(out: &mut String, heading: &str, body: &str) {
    out.push_str(&format!("## {heading}\n\n"));
    out.push_str(body);
    out.push_str("\n\n");
}

fn oneline(s: &str) -> String {
    s.lines().collect::<Vec<_>>().join(" ").trim().to_string()
}

/// Translate a Claude-Code-shape pattern (e.g. `dwarven --actor spec issue
/// view:*`) into opencode's shape (`dwarven --actor spec issue view*`).
/// Strip the trailing `:*` and replace with `*`.
pub fn to_opencode_pattern(claude_pattern: &str) -> String {
    if let Some(stripped) = claude_pattern.strip_suffix(":*") {
        format!("{stripped}*")
    } else {
        claude_pattern.to_string()
    }
}

/// Inverse view for display in the prompt body.
fn strip_claude_pattern(claude_pattern: &str) -> String {
    claude_pattern.replace(":*", "*")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_translation_strips_claude_trailing() {
        assert_eq!(
            to_opencode_pattern("dwarven --actor spec issue view:*"),
            "dwarven --actor spec issue view*"
        );
        assert_eq!(to_opencode_pattern("git status:*"), "git status*");
    }

    #[test]
    fn pattern_translation_passes_non_claude_unchanged() {
        assert_eq!(to_opencode_pattern("git status"), "git status");
        assert_eq!(to_opencode_pattern("*"), "*");
    }
}
