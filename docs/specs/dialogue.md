---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Dialogue

This document specifies how agents and the maintainer communicate. Two modes are supported: **interactive** (a dispatched agent asks a synchronous question via the host's question mechanism) and **detached** (an agent escalates by transitioning to `state: maintainer` and posting a comment; the maintainer responds asynchronously via the web UI; a subsequent dispatch resumes).

This spec composes pieces defined elsewhere (`work-states.md`, `dwarven-cli.md`, `web-ui.md`, `host-adapter.md`). It defines the protocol; the underlying mechanisms live in those documents.

---

## R1 — Scope

R1.1 — This spec defines: the two dialogue modes; the agent-side and maintainer-side flows in each; the convention for structured questions; and which agents may use which mode.

R1.2 — Out of scope: implementation of the host's interactive-question mechanism (`host-adapter.md`); on-disk format of dialogue artifacts (`storage-model.md`); the UI screens that surface dialogue (`web-ui.md`).

---

## R2 — The two modes

R2.1 — **Interactive mode.** The agent is dispatched in a live shell session (the maintainer is at the keyboard). The agent may ask the maintainer a question via the host's native question mechanism (Claude Code's `AskUserQuestion`; the opencode equivalent). Answers arrive synchronously within the agent's session.

R2.2 — **Detached mode.** The agent is dispatched non-interactively (by hook, scheduled task, or CI). No human is at the keyboard. The agent must not block waiting for input. Escalation happens by transitioning the issue to `state: maintainer`, setting a `blocker`, posting a structured comment, and exiting. The maintainer responds asynchronously via the web UI; a subsequent dispatch reads the new state and continues.

R2.3 — Mode is a property of the dispatch context, not of the agent itself. The same agent definition may be invoked interactively or detachedly; its system prompt distinguishes which behaviors it may exhibit (R8).

R2.4 — Detached mode is a v0.2+ scope marker in the inherited v0.1 design. In the v2 architecture, detached dispatch lands in v1 (alongside the hub) — the prerequisite mechanisms (state machine, hub comments, web UI) are all v1 deliverables.

---

## R3 — Interactive dialogue

R3.1 — Within a live agent session, the agent uses the host-native question mechanism to ask the maintainer one question at a time. The host's adapter (`host-adapter.md`) maps this to the host primitive (`AskUserQuestion` in Claude Code).

R3.2 — Questions follow the discipline established in v0.1 prompts: one question at a time; 2–3 alternatives plus the agent's lean; the question stays open until answered.

R3.3 — Interactive answers are not automatically persisted to hub comments. The agent that received the answer may, at its discretion, summarize the exchange as a hub comment if the decision is load-bearing for downstream work. (Rationale: most interactive Q&A is conversational; persisting every clarification would clutter the issue thread.)

R3.4 — Decisions reached interactively that change the issue state, frontmatter, or dependencies must still be recorded via the hub mutation paths (`dwarven` CLI). The interactive question itself is ephemeral; the resulting state change is persistent.

---

## R4 — Detached dialogue protocol

R4.1 — When a detached agent reaches a decision point requiring maintainer input, it must perform the following sequence atomically:

1. Post a comment to the issue summarizing what is needed (R5).
2. Set `blocker: maintainer-input` on the issue (`dwarven issue blocker set`).
3. Transition the issue to `state: maintainer` (`dwarven issue transition`).
4. Exit cleanly (no further work attempted on the issue).

R4.2 — The order matters: the comment must precede the state transition so that the maintainer, upon seeing the issue in their Inbox (`web-ui.md#R4`), finds the question waiting at the top of the comment thread.

R4.3 — A detached agent must never block waiting for input. If the agent reaches a question it cannot answer alone, the only legitimate exit path is detached escalation (R4.1).

R4.4 — A detached agent must never use the host's interactive question mechanism. The host adapter (`host-adapter.md`) is responsible for ensuring that interactive primitives are unavailable or explicitly suppressed in detached dispatch contexts.

---

## R5 — Agent escalation comment

R5.1 — The escalation comment posted in R4.1 step 1 should follow this structure:

```markdown
**Question.** <One-sentence framing of what is needed.>

**Context.** <Brief recap of why this question arose: what the agent was doing, what it found, what is unclear or under-decided.>

**Alternatives.**
1. <Option A — short description, including consequences.>
2. <Option B — short description.>
3. <Option C — short description.>

**My lean.** <The agent's recommendation and the single most important reason.>
```

R5.2 — The structure in R5.1 is a convention, not a hard validation rule. The hub does not parse the comment body; the maintainer interprets it. The web UI may render it with mild emphasis (e.g., bolding the section headers) but does not depend on the structure being present.

R5.3 — Free-form questions (without alternatives) are permitted when the question genuinely has no enumerable options. The expected default is the structured form: enumeration disciplines the agent's framing and gives the maintainer a faster decision path.

R5.4 — The escalation comment is a normal `kind: comment` (`storage-model.md#R4.4`). The fact that it is a question is observable from the issue's resulting state (`state: maintainer`, `blocker: maintainer-input`) — no separate comment kind is introduced in v1.

---

## R6 — Maintainer response

R6.1 — The maintainer responds to a detached escalation via the web UI's Issue Detail screen (`web-ui.md#R6`). Steps:

1. Read the agent's escalation comment.
2. Post one or more reply comments (R6.2 conventions).
3. Clear the blocker.
4. Transition the issue to the appropriate active state (typically back to the asking agent's state, but the maintainer may route elsewhere).

R6.2 — Reply convention: the maintainer's comment should answer the agent's question directly, citing alternative numbers from the structured form (R5.1) when applicable. Free-form prose is acceptable.

R6.3 — Operations 2–4 in R6.1 should be performed close together so the issue does not linger in `state: maintainer` after the maintainer has decided.

R6.4 — The maintainer may also use the CLI for these operations (`dwarven issue comment`, `dwarven issue blocker clear`, `dwarven issue transition`); the web UI is the recommended surface but not the only one.

R6.5 — The maintainer may decline to answer (e.g., "this question is no longer relevant; dropping the issue"). In that case the appropriate transition is `state: dropped` with a closure comment explaining why (`dwarven-cli.md#R6.7`).

---

## R7 — Continuation: resuming after escalation

R7.1 — The next dispatch of an agent on the resumed issue (whether interactive or detached) reads the issue's current state and the comment thread, including the maintainer's reply.

R7.2 — Agents must inspect the most recent state-change comment when picking up an issue. If the issue was previously in `state: maintainer` and is now active again, the agent reads the comments since the last state-change to find the maintainer's reply.

R7.3 — There is no automatic re-dispatch in v1: the maintainer (or a future scheduled task) explicitly invokes the next agent via slash command. Re-dispatch on `state` change is a v0.2+ marker in the inherited design and tracks under the broader detached-dispatch automation (out of scope for this spec).

R7.4 — If an agent picks up an issue and finds the maintainer's reply ambiguous or insufficient, it re-escalates via R4 — a fresh question, a new escalation comment.

---

## R8 — Which agents may use which modes

R8.1 — All agents must support detached mode (R2.2). An agent that cannot tolerate detached dispatch is an architectural defect.

R8.2 — A subset of agents additionally supports interactive mode and is permitted to use the host's interactive question mechanism. Interactive-permitted agents are the *dialogue agents*: Spec, Architect, Gap, PM, Planning.

R8.3 — Discrete-work agents — Test Dev, Implementation, Code Review, Doc, Triage — must not use the host's interactive question mechanism even when dispatched in an interactive shell. They escalate structurally via R4. Rationale: their work is mechanical enough that a question signals an upstream gap; the right resolution is to transition the issue to a dialogue agent, not to interrogate the maintainer mid-task.

R8.4 — The host adapter (`host-adapter.md`) implements R8.2–R8.3 by gating the interactive primitive in each agent's tool allowlist. In Claude Code: `AskUserQuestion` appears in the allowlist for dialogue agents only.

R8.5 — The maintainer's shell (the host's top-level session) may use the interactive primitive freely; it is the maintainer's own surface, not an agent.

---

## R9 — Out of scope for v1

R9.1 — **Multi-turn detached dialogue per dispatch.** A detached agent escalates once per dispatch and exits. Continuation is a fresh dispatch (R7).

R9.2 — **Auto re-dispatch on state change.** v0.2+ detached-dispatch automation is out of scope for this spec.

R9.3 — **Structured machine-readable questions.** Questions are Markdown prose with a recommended structure (R5.1). No JSON-schema'd question artifact in v1.

R9.4 — **Maintainer-to-agent broadcast.** The maintainer cannot send a message to an agent that is not currently in the maintainer-state queue. Communication happens through issues; if the maintainer wants to redirect an agent's work, they file or modify an issue.

R9.5 — **Cross-issue conversations.** Each dialogue is scoped to a single issue's comment thread. Questions that span multiple issues are out of scope; the agent should pick the most-relevant issue or escalate as a meta-question on a parent issue.
