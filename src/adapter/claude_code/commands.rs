//! `.claude/commands/<name>.md` slash-command files. Each is a thin
//! dispatch into the corresponding subagent.

use crate::adapter::claude_code::agents::roster;

#[derive(Debug, Clone, Copy)]
pub struct CommandDef {
    pub name: &'static str,
    pub agent: &'static str,
    pub description: &'static str,
}

pub const COMMAND_NAMES: &[&str] = &[
    "spec", "architect", "gap", "pm", "plan", "test", "implement", "review", "doc", "triage",
];

pub fn list() -> Vec<CommandDef> {
    roster()
        .iter()
        .map(|a| CommandDef {
            name: a.name,
            agent: a.name,
            description: a.description,
        })
        .collect()
}

pub fn render(cmd: &CommandDef) -> String {
    format!(
        "---\n\
         description: {desc}\n\
         allowed-tools: Agent\n\
         ---\n\
         \n\
         Dispatch the **{agent}** agent. Forward the maintainer's argument verbatim:\n\
         \n\
         ```\n\
         Agent(subagent_type=\"{agent}\", prompt=\"{{$ARGUMENTS}}\")\n\
         ```\n\
         \n\
         See `docs/specs/agent-roster.md` for the agent's full contract.\n",
        desc = cmd.description,
        agent = cmd.agent,
    )
}
