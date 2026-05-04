# PRD: Praxis — Personal AI Agent Framework

## Problem Statement

The user wants a personal AI agent that can theoretically do anything AI is capable of. The core thesis: with only three elements — LLM resources, a good ReAct loop, and enough tools/skills — an AI can accomplish any task. Different scenarios only require different skill combinations loaded into the same universal loop.

Existing agent frameworks (LangChain, CrewAI, etc.) are bloated, opinionated, and hard to understand at a deep level. The user wants full control and understanding by building from scratch.

## Solution

Praxis (πράξις — "action, practice") is a personal AI agent framework built from scratch in Rust. It features a two-dimension capability model:

- **Knowledge (Agent Skills / SKILL.md)**: Markdown documents conforming to the Agent Skills spec (agentskills.io) that tell the model "how to think and what to do." These are injected into the model's context as knowledge references. Skills describe workflows, strategies, conventions, and domain knowledge.

- **Capability (Tools / function-calling)**: Structured operations with JSON Schema definitions that the model can explicitly invoke. Tools are atomic, efficient, and precise.

A unified ReAct loop (think → act → observe → repeat) orchestrates both, with a thin CLI frontend and a pluggable provider layer for different LLMs.

## User Stories

1. As a developer, I want to define a "coder" profile that loads coding-related skills and tools, so I can have an AI assistant that helps me write and review code
2. [V2] As a user, I want to switch between profiles (e.g., "coder" → "researcher") without restarting the agent, so I can use different skill sets for different tasks
3. As a skill author, I want to create a new skill by writing a SKILL.md file with YAML frontmatter, so I can define knowledge-based capabilities without writing Rust code
4. As a skill author, I want the agent to read my SKILL.md content to inform its decisions, so it can follow domain-specific strategies and workflows
5. As a tool developer, I want to implement a new function-calling tool by writing a Rust struct that implements the Tool trait, so I can add precise, schema-validated capabilities
6. As a user, I want the agent to use function-calling tools for structured operations (reading files, creating issues, searching code), so actions are reliable and predictable
7. As a developer, I want to swap LLM providers (OpenAI, Anthropic, local models) without changing the agent's loop logic, so I can choose the best model for my needs
8. As a user, I want the agent's loop to handle errors gracefully (tool failures, API timeouts) and attempt recovery, so it doesn't crash on transient issues
9. As a user, I want the agent to support MCP (Model Context Protocol) tools in the future, so I can connect to external tool servers without writing Rust code
10. As a user, I want to run the agent from CLI, and potentially from TUI/API in the future, so it fits different usage scenarios with a shared core
11. As a user, I want session management to save and resume conversations, so I can pause and continue long-running tasks
12. As a user, I want the agent to automatically manage context window limits (truncation, summarization), so it doesn't fail on very long conversations
13. [V2] As a skill author, I want to optionally bundle function-calling tools with a skill (via a tools manifest), so complex skills can provide both knowledge and dedicated capabilities
14. [V2] As a user, I want the agent to load skill resources (scripts, reference files) on demand with progressive disclosure, so context window isn't wasted on unused resources
15. As a user, I want streaming output from the agent, so I can see the model's thinking and actions in real-time
16. As a developer, I want the framework to be well-tested with unit and integration tests, so I can refactor with confidence
17. As a user, I want the agent's core (session, config, profile, loop) to be independent of any specific frontend, so others can build custom interfaces on top
18. As a user, I want to compose profiles that combine multiple skills and tools, selecting the right knowledge and capabilities for specific scenarios
19. As a skill author, I want the framework to support the Agent Skills specification (agentskills.io) for SKILL.md format, so my skills are interoperable with other agent platforms

## Implementation Decisions

**Language & Approach**: Rust, built from scratch (no LangChain, CrewAI, or similar frameworks).

**Two-dimension capability model**:
- Agent Skills (knowledge): SKILL.md files per agentskills.io spec, with YAML frontmatter + Markdown body, loaded into the model's system prompt as context
- Tools (capability): Function-calling interface with JSON Schema, implemented via a Tool trait in Rust

**Implementation strategy**: Tracer-bullet — build a runnable end-to-end slice first, then layer in real implementations. Every step produces a working binary.

**Build order**:
1. `protocol` types + stub loop (hardcoded provider returning "hi") — runnable
2. Real OpenAI provider adapter (streaming via chunks)
3. One real tool (bash), wired into loop
4. Skill loading from external directory, injected into system prompt
5. Session save/resume
6. CLI polish

**Module architecture**:

| Module | Responsibility |
|--------|---------------|
| `protocol` | Shared data types — Message, ContentBlock, Role, ToolDef. Pure data, Serde-enabled |
| `providers` | Provider trait + LLM adapter implementations (OpenAI first). Returns a stream of chunks; the loop assembles the final Message |
| `tools` | Tool trait + ToolRegistry + built-in tools (filesystem, shell, web). MCP adapter lives here as a Tool implementation |
| `skills` | Skill loader (parses SKILL.md frontmatter + body), skill registry, context injection into system prompt. Follows Agent Skills spec |
| `loop` | ReAct engine (think → act → observe → repeat), context window management, error recovery, streaming support |
| `core` | Session management, config loading (YAML), profile resolution, main entry point. Frontend-agnostic |
| `cli` | Thin CLI wrapper. Parses args, calls core, displays output. One of many possible frontends |

**Key design rules**:
- The ReAct loop sees Tools (from ToolRegistry) as its only capability interface. It does NOT know about skills
- Skills produce context for the model to read. In V1, skills and tools are orthogonal — no skill can declare or bundle a tool
- MCP is a Tool trait implementation, not a separate layer. `tools/mcp/` contains the MCP client and adapter
- Core encapsulates session, config, and profile. CLI is a thin shell over Core
- V1: all skill bodies are loaded at startup. Progressive disclosure is a future enhancement

**Provider abstraction** (returns a stream of chunks; the ReAct loop assembles the final Message):
```rust
trait Provider {
    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolDef>) -> Result<ChatStream>;
}
```

**Tool abstraction**:
```rust
trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<String>;
}
```

## Design Decisions (resolved during grill)

**Skill/Tool orthogonality**: V1 skills are knowledge-only. Tools are independent compiled structs. A profile selects both independently. See ADR-0001.

**Skill location**: External directory — `~/.config/praxis/skills/<slug>/SKILL.md`. One directory per skill, loaded at startup.

**Tool location**: Compiled into the binary as Rust structs implementing the Tool trait. Selected by `name()` string in the profile YAML.

**Profile YAML schema**: A profile includes: `name`, `system_prompt`, `model`, `provider`, `skills: [list]`, `tools: [list]`. Profiles are static, chosen at session start.

**Provider credentials**: API keys come from environment variables (e.g. `OPENAI_API_KEY`), not from config files.

**Streaming**: The Provider trait returns a `ChatStream` (stream of chunks). The ReAct loop drives the stream, assembles the final Message, and forwards chunks to the frontend for display.

**Multi-tool calls**: When the model calls multiple tools in one response, they execute sequentially — one tool result per model turn. Parallel execution is a future optimization.

**Tool error handling**: `execute()` returns `Result<String>`. `Ok` = tool ran (even if the output text describes an error). `Err` = tool could not run at all (code bug, system failure). Both are fed back as tool result messages for the model to handle.

**ReAct loop error handling**: The loop is a dumb pipe. Tool failures, malformed tool calls, and invalid model output are fed back as messages — the model decides how to recover. Only provider connection failures are retried internally (3 attempts, exponential backoff, inside the provider adapter).

**Session persistence**: Sessions save the full message history (including tool results) as JSONL in `~/.config/praxis/sessions/`. On resume, history is used as-is — tool calls are never re-executed.

**Context window management (V1)**: All skill bodies are injected at startup. When approaching the model's token limit, the loop drops the oldest non-system messages. No summarization in V1.

**ReAct loop guard**: Configurable per profile (`max_iterations`). Prevents infinite loops. V1 default: 50.

**Profile switching**: V1: no mid-session switching. Switching profiles ends the current session and starts a new one. The new session can include a summary of the prior session as its first user message.

## Testing Decisions

- Each module should have unit tests that test external behavior, not implementation internals
- What makes a good test: given X input and Y environment setup, expect Z observable output. Test through public interfaces only
- Provider adapters: tested with mock HTTP servers (wiremock or similar)
- ReAct loop: tested with mock Provider and mock Tool implementations to verify loop behavior without real API calls
- Tool implementations: tested against real or temporary resources (temp files for filesystem tools, mock server for web tools)
- Skill loader: tested with fixture SKILL.md files covering valid and invalid frontmatter
- CLI: integration tests that run the binary and assert on stdout/stderr

**Modules requiring test coverage (high priority)**:
- `protocol` — serialization round-trips
- `providers` — adapter translation correctness
- `tools` — registry operations and built-in tool behavior
- `loop` — ReAct flow correctness, error recovery paths
- `skills` — SKILL.md parsing and validation

## Out of Scope

- GUI, TUI, or Web frontend (CLI only for v1)
- MCP server implementation (MCP client/adapter only, future milestone)
- Multi-agent collaboration or agent-to-agent communication
- Local model hosting or inference (use external API providers)
- Built-in skill package registry or marketplace
- Real-time voice or video interaction
- Dynamic tool loading (tools are compiled into the binary in V1; MCP adapter is a future milestone for external tools)

## Further Notes

- The repo is currently empty (only agent docs scaffolding). This PRD defines the full initial scope
- After PRD approval: implement using tracer-bullet strategy — get a runnable end-to-end slice working first, then layer in real providers and tools
- The name "Praxis" reflects the core philosophy: theory meeting practice — an agent that doesn't just think, but actually does
