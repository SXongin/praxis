# Praxis

A personal AI agent framework: LLM + ReAct loop + pluggable skills and tools, built from scratch in Rust.

## Language

**Skill**:
Knowledge injected into the model's context as a SKILL.md file (per agentskills.io spec). Describes workflows, strategies, conventions, and domain knowledge. Skills are read-only context — they do not execute code.
_Avoid_: Plugin, extension, capability, tool

**Tool**:
A function-calling capability the model can explicitly invoke at runtime. Defined by a Rust struct implementing the Tool trait, with a JSON Schema for arguments and a structured result. Tools are independent of skills.
_Avoid_: Skill, plugin, action, command

**Profile**:
A named, static YAML configuration that selects skills (context), tools (capabilities), a system prompt, a model choice, and provider settings. Chosen at session start.
_Avoid_: Persona, preset, mode

**Session**:
One run of the ReAct loop under a specific **Profile**. Has its own message history (including tool results) and can be saved/resumed. On resume, stored history is used as-is — tool calls are never re-executed. A profile can have many sessions.
_Avoid_: Conversation, chat, run

**ReAct loop**:
The core engine: think → act → observe → repeat. Sends messages to a **Provider**, receives a stream of chunks, executes **Tools** when the model calls them, and feeds results back. Dumb pipe — the model is the intelligence.
_Avoid_: Agent loop, reasoning loop, execution engine

**Provider**:
An adapter that translates the loop's message stream into an LLM-specific API call (OpenAI, Anthropic, etc.). Returns a stream of chunks. Credentials come from environment variables.
_Avoid_: Driver, backend, LLM client

## Relationships

- A **Session** runs under exactly one **Profile**, chosen at session start
- A **Profile** selects which **Skills** and **Tools** are active
- **Skills** provide context for the model to read; **Tools** provide capabilities for the model to invoke
- In V1, **Skills** and **Tools** are orthogonal — no skill can declare or bundle a tool
- Profile switching means ending the current **Session** and starting a new one (V1: no mid-session switching)
- Tool execution errors and invalid model output are fed back as messages into the ReAct loop for the model to handle. Only provider connection failures are retried internally.
- The Provider trait returns a stream of chunks; the ReAct loop assembles the final Message and forwards chunks to the frontend for display
- Skills live in an external directory: `~/.config/praxis/skills/<slug>/SKILL.md` (one directory per skill)
- Tools are compiled into the binary (Rust structs). A **Profile** selects which tools to activate by their `name()` string
- A **Profile** YAML includes: name, system prompt, model, provider, skills list, tools list, max_iterations (optional, default 50)
- When the model calls multiple tools in one response, they execute sequentially — one tool result per model turn
- Sessions are saved as JSONL in `~/.config/praxis/sessions/`

## Example dialogue

> **Dev:** "If I load the 'github-triager' skill, does the agent automatically get a `github-create-issue` tool?"
> **Domain expert:** "No. Skills are knowledge, Tools are capability. You'd load the skill for context AND separately register the tool in your profile."

## Flagged ambiguities

- "skill" was initially considered as a package that could bundle tools. Resolved: V1 skills are knowledge-only. Tool bundling by skills is a future concern.
