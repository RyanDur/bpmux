# OpenCode and Open-Source AI Coding Assistants

Research into OpenCode and the broader landscape of open-source AI coding tools,
evaluated through the lens of bpmux's needs.

## 1. What Is OpenCode?

**Repository**: https://github.com/opencode-ai/opencode (archived Sept 2025)
**Active fork**: https://github.com/anomalyco/opencode (v1.3.13 as of April 2026)
**Website**: https://opencode.ai
**Docs**: https://opencode.ai/docs/
**License**: Open source
**Stars**: ~136k GitHub stars

OpenCode is a terminal-based AI coding agent. It provides a TUI for interacting
with LLMs to read, write, search, and execute code in your project directory.
Think "Claude Code but open source and provider-agnostic."

### Key capabilities

- Interactive TUI with Plan mode (read-only analysis) and Build mode (full
  editing)
- Multi-provider: OpenAI, Anthropic, Google Gemini, AWS Bedrock, Groq, Azure
  OpenAI, OpenRouter, local models
- MCP server support (both local stdio and remote HTTP)
- LSP integration for diagnostics feedback after edits
- Session persistence in SQLite
- Auto-compaction when approaching context limits (triggers at ~90% of window)
- Git-based snapshots for undo/redo (uses `git write-tree` / `git read-tree`)
- Subagent architecture for delegating specialized tasks
- Desktop app, IDE extension, and CLI modes

### How it differs from Claude Code

| Aspect | Claude Code | OpenCode |
|--------|-------------|----------|
| Provider lock-in | Anthropic only | Any provider |
| Source | Proprietary | Open source |
| Language | TypeScript/Node | TypeScript/Bun (Go TUI shell) |
| MCP | Client + server | Client only |
| System prompt | Opaque | Fully visible and customizable |
| Agent model | Single agent + subagents | Configurable multi-agent |
| Cost control | Anthropic pricing | BYOK, local models possible |

### How it differs from Aider

Aider (https://github.com/Aider-AI/aider) is Python-based and focuses on
git-aware pair programming with a simpler CLI (no TUI). Aider's core loop is
edit-commit-repeat. OpenCode is broader: TUI, multiple agents, LSP, MCP, session
management.

### How it differs from Cursor

Cursor is a proprietary VS Code fork with AI deeply integrated into the editor
GUI. OpenCode is terminal-first and open source. Different philosophies:
Cursor optimizes for IDE users; OpenCode optimizes for terminal users who want
provider freedom.

## 2. Architecture Deep Dive

Source: https://cefboud.com/posts/coding-agents-internals-opencode-deepdive/

### Runtime and code structure

The original Go implementation was rewritten in TypeScript running on Bun.
The Go binary remains as the TUI shell, but the core agent logic is now
TypeScript using the Vercel AI SDK for provider abstraction.

Architecture is split:
- **Backend**: Bun runtime + Hono HTTP server with OpenAPI endpoints
- **TUI**: Go binary using Bubble Tea, launched by the Bun process
- **Communication**: SSE on `/sse` endpoint for real-time updates
- **Client SDK**: Auto-generated from OpenAPI spec via Stainless

### The agentic loop

Uses the AI SDK's `streamText` in a continuous loop:

1. Assemble system prompt (layered: provider header, environment, AGENTS.md,
   agent-specific prompt)
2. Send messages + tool definitions to LLM
3. Process stream events: text chunks, tool calls, tool results, errors
4. For each tool call: validate schema (Zod), check permissions, execute,
   return result
5. Loop until: model stops calling tools, 1000 steps reached, or user
   interrupts

### System prompt construction

Prompts are assembled from multiple layers:
1. Provider-specific header (Anthropic vs OpenAI vs Gemini variants)
2. Environment block (model name, working directory, platform, date)
3. Custom instructions from `AGENTS.md` or `CLAUDE.md` files (walks filesystem)
4. Agent-specific prompt (build, plan, explore, etc.)
5. User override via `--system` flag

Mode switching between Plan and Build uses a synthetic system message called
`BUILD_SWITCH` to explicitly notify the model its capabilities changed.

### Tool system

13 built-in tools:
- **File ops**: read, write, edit, apply_patch
- **Search**: grep, glob, list
- **Execution**: bash (with permission gating)
- **Intelligence**: lsp (definitions, references, hover, call hierarchy)
- **External**: webfetch, websearch
- **Meta**: question (ask user), todowrite, skill (load SKILL.md files)

MCP tools are discovered at startup and merged into the same registry.
All tools go through permission checks, plugin hooks
(`tool.execute.before`/`after`), and output truncation.

### Session and context management

- SQLite for persistent storage of conversations and sessions
- Messages stored with metadata: tool calls, results, timestamps, costs
- Auto-compaction at 90% of context window: generates summary preserving
  file paths, recent changes, and next steps
- Git-based snapshots for working state (non-destructive, uses object refs)

### Subagent architecture

Subagents are invoked via a `task` tool:
- Accepts task description + target agent type
- Spins up isolated session with independent context
- Restricted tool access per agent config
- Returns only final text; internal tool calls hidden from parent
- Enables recursive composition (agents delegating to agents)

Built-in agents:
- **build**: Default, full tool access
- **plan**: Read-only analysis, edits restricted to plan files
- **general**: Subagent for multi-step tasks, no todo tools
- **explore**: Read-only codebase explorer
- **compaction/title/summary**: Hidden system agents

### MCP integration details

Docs: https://opencode.ai/docs/mcp-servers/

Two transport types:
- **Local**: `type: "local"`, spawns process via command+args, communicates
  over stdio
- **Remote**: `type: "remote"`, connects via HTTP URL with optional auth headers

MCP servers are spawned at startup. Tools discovered via standard MCP
initialization, added to registry alongside built-in tools. OAuth support
for remote servers (Dynamic Client Registration, RFC 7591).

Configuration in `opencode.jsonc`:
```json
{
  "mcp": {
    "my-server": {
      "type": "local",
      "command": ["npx", "my-mcp-server"],
      "environment": { "API_KEY": "..." },
      "timeout": 5000
    }
  }
}
```

## 3. What Can bpmux Learn From OpenCode?

### Patterns worth adopting

**Layered system prompt assembly.** OpenCode's approach of composing prompts
from environment + project instructions + agent-specific directives is clean.
If bpmux builds a bot participant, this layered approach lets you inject session
context (who's in the chat, what's been discussed, what files are relevant)
without rewriting the whole prompt.

**Tool permission gating.** Every tool call goes through a permission check
before execution. For bpmux, where multiple AI participants might be acting,
permission control is essential. You don't want a bot participant running
`rm -rf` without approval.

**Auto-compaction.** Monitoring token usage and summarizing when approaching
limits is a pattern bpmux needs regardless of whether it builds its own bot.
Long-running pair programming sessions will hit context limits.

**Subagent delegation.** The pattern of a primary agent spawning specialized
subagents (one for exploration, one for editing, one for planning) maps well
to bpmux's multi-participant model. Different bots could specialize.

**Git-based snapshots.** Using `git write-tree` for non-destructive state
capture is clever. Cheaper than full commits, allows rollback without
polluting history. Useful for any tool-executing bot.

### Patterns to avoid

**Monolithic agent binary.** OpenCode bundles everything: TUI, agent loop,
tool execution, provider abstraction. For bpmux, the agent should be a
participant in the chat, not the whole application. Keep separation clean.

**Provider abstraction complexity.** OpenCode supports 7+ providers with
different prompt formats, tool calling conventions, etc. bpmux's bot doesn't
need this. Pick one provider (or use a simple abstraction) and keep it lean.

## 4. MCP Support and bpmux Connectivity

### OpenCode as MCP client

OpenCode supports MCP as a client (it connects to MCP servers). It does NOT
expose itself as an MCP server.

If bpmux exposes an MCP server (which it could for session context, message
history, participant info), OpenCode could connect to it. The user would
configure bpmux's MCP server in their `opencode.jsonc`:

```json
{
  "mcp": {
    "bpmux": {
      "type": "local",
      "command": ["bpmux", "mcp-server"],
      "timeout": 10000
    }
  }
}
```

Or for remote:
```json
{
  "mcp": {
    "bpmux": {
      "type": "remote",
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

This would give OpenCode's AI access to bpmux session data (messages,
participants, context) via MCP tools.

### OpenCode as a bpmux participant

OpenCode can NOT act as a bpmux participant directly. It's designed as a
human-facing tool, not as a programmatic agent. It has no API for receiving
messages from an external system and responding.

However, the `/sse` endpoint and Hono HTTP server suggest it could be
extended. The OpenAPI spec means you could programmatically send messages
to an OpenCode session. This is fragile and not the intended use.

## 5. Build vs. Integrate: Should bpmux Build Its Own Coding AI?

This is the core strategic question. Here's the honest tradeoff analysis.

### Option A: Integrate existing tools (OpenCode, Claude Code, Aider, etc.)

**How it works**: bpmux stays as communication infrastructure. AI coding tools
connect via MCP. bpmux provides session context; the tool provides coding
capability.

**Pros**:
- Dramatically less code to write and maintain
- Leverage years of tool development (OpenCode has 136k stars, hundreds of
  contributors)
- Users choose their preferred tool
- Provider-agnostic by default
- bpmux stays focused on its core value: shared reasoning and communication

**Cons**:
- No control over the AI's behavior in the session
- AI tools are designed for 1-human-1-AI interaction, not group chat
- MCP gives context but doesn't give turn-taking, awareness of other
  participants, or session norms
- The AI can't "see" other AI participants' reasoning or respond to it
- Integration friction: each tool has different config, different capabilities

**Verdict**: Good enough for v1. MCP server gives context to external tools.
But the experience will be "a human using Claude Code that happens to have
bpmux context" rather than "an AI participant in a group programming session."

### Option B: Build a minimal coding bot

**What "minimal" means**:
1. LLM API client (call Anthropic/OpenAI directly)
2. Tool execution layer (file read/write, grep, bash)
3. System prompt with session awareness (who's here, what's been discussed)
4. Message loop: receive message from bpmux, think, respond, execute tools
5. Permission system (human approval for destructive actions)

**Estimated components** (based on OpenCode's architecture):
- LLM client: ~500 lines (single provider, streaming)
- Tool definitions + execution: ~1000 lines (read, write, edit, bash, grep)
- System prompt assembly: ~200 lines
- Session integration: ~300 lines (reading bpmux messages, posting responses)
- Permission gating: ~200 lines
- **Total: ~2000-2500 lines of Rust**

**Pros**:
- Full control over behavior in group context
- Can implement turn-taking, awareness of other participants
- Can respond to other AIs' reasoning (the core bpmux thesis)
- Can enforce session norms (ask before editing, announce intentions)
- Tight integration with bpmux's message format and session model

**Cons**:
- Significant development effort
- Need to maintain tool implementations
- LSP integration alone is substantial work
- Risk of building a worse version of what already exists
- Scope creep: "just one more tool" is a gravity well

**Verdict**: Worth building IF bpmux's thesis requires AI-to-AI interaction
that external tools can't provide. The minimal version is tractable (~2500
lines). The risk is scope creep toward rebuilding OpenCode.

### Option C: Hybrid approach (recommended)

**Phase 1**: Build bpmux MCP server so external tools can connect. This is
pure infrastructure work and validates the communication model.

**Phase 2**: Build a thin bot participant that wraps an existing tool's
capabilities. Use Claude Code's or OpenCode's tool patterns but run them
inside bpmux's participant model. The bot reads bpmux messages, formulates
its own context, calls an LLM, and posts responses back to the session.

**Phase 3**: Only if needed, build specialized tools. Start with read/grep
(the most common operations). Add edit/bash only when the lack of them
blocks real usage.

This avoids the "rebuild everything" trap while still giving bpmux the
AI-as-participant model that external tools can't provide.

## 6. Landscape of Open-Source Coding AI Tools

### Tools that could connect to bpmux via MCP

| Tool | Language | MCP Support | Headless/API | Could be bpmux participant? |
|------|----------|-------------|--------------|---------------------------|
| **OpenCode** | TS/Go | Client (local + remote) | No native API | No (human-facing only) |
| **Claude Code** | TS | Client + server | CLI mode | Partial (CLI piping) |
| **Aider** | Python | Via community MCP server | CLI, scriptable | Partial (stdin/stdout) |
| **Cline** | TS | Client + can create servers | CLI 2.0 headless mode | Yes (headless + stdin/stdout) |
| **Continue** | TS | Client | CLI headless mode (`cn -p`) | Yes (headless mode) |
| **Goose** | Rust | Client (3000+ servers) | CLI + desktop | Partial |
| **Cody** | TS | Client | VS Code extension only | No |
| **Zed AI** | Rust | Client | Editor only | No |

### Detailed notes on key tools

**Cline** (https://github.com/cline/cline)
- Most promising for bpmux integration
- CLI 2.0 has headless mode (`-y` flag): no interactive UI, streams to stdout
- Full stdin/stdout support: can pipe content in, chain commands
- ACP (Agent Client Protocol) support for editor-agnostic operation
- MCP client with ability to create custom MCP servers
- 5M+ developers, very active development

**Goose** (https://github.com/block/goose)
- Built by Block (Square), written in Rust
- Strong MCP-first architecture (connects to 3000+ servers)
- Contributed to Linux Foundation's Agentic AI Foundation alongside MCP itself
- Apache 2.0 license
- 27k+ stars, 350+ contributors
- CLI and desktop modes

**Continue** (https://github.com/continuedev/continue)
- Headless mode: `cn -p "prompt"` outputs final response only
- API key auth for CI/CD environments
- MCP support in agent mode
- Good for automation but designed as code review/quality tool, not coding agent

**Aider** (https://github.com/Aider-AI/aider)
- Python, CLI-focused, git-aware
- Community MCP servers wrap it for WebSocket/JSON-RPC access
- Simpler architecture than OpenCode but very effective for edit-commit loops
- No native MCP client, but can be wrapped as MCP server

### Tools with WebSocket or custom transport support

Most tools use stdio for MCP. For bpmux's P2P model, you need either:

1. **WebSocket**: Aider has community MCP servers with WebSocket transport.
   MCP itself supports HTTP/SSE and is moving toward WebSocket.
2. **HTTP/SSE**: OpenCode remote MCP, Continue remote, standard MCP transport.
3. **stdio**: All tools support this. bpmux could spawn a local process and
   communicate over stdio, then bridge to the P2P network.

The most practical path: bpmux exposes an MCP server over stdio (for local
tools) and HTTP/SSE (for remote tools). Any MCP-compatible tool connects.

## 7. Concrete Recommendations for bpmux

### Immediate (this week)

1. **Don't build a bot yet.** The MCP server is the right first step. It gives
   bpmux connectivity to the entire ecosystem without committing to a bot
   architecture.

2. **Study OpenCode's tool definitions.** The 13 built-in tools with Zod
   schemas are a good reference for what a coding bot needs. Don't implement
   them yet, but understand the surface area.

3. **Study OpenCode's system prompt layering.** If/when bpmux builds a bot,
   the prompt needs session context. OpenCode's pattern of environment +
   instructions + agent prompt is a good template.

### Medium-term (if bot is needed)

4. **Start with the thinnest possible bot.** A participant that:
   - Receives messages from bpmux session
   - Calls an LLM API with session context as system prompt
   - Posts responses back to session
   - NO tool execution in v1 (just conversation)

5. **Add read-only tools first.** File read, grep, glob. Let the bot
   understand the codebase without modifying it. This is the "explore" agent
   pattern from OpenCode.

6. **Add write tools only with human approval.** Follow OpenCode's permission
   model: `ask` by default, `allow` only for safe operations.

### Strategic

7. **The real differentiator isn't the bot.** It's the shared session.
   OpenCode, Claude Code, Aider -- they're all single-agent tools. bpmux's
   value is the multi-participant model. A mediocre bot in a great session
   architecture beats a great bot running in isolation.

8. **Watch Cline CLI 2.0.** Its headless mode with stdin/stdout is the closest
   thing to "an existing tool that could be a bpmux participant." If someone
   else solves the coding agent problem, bpmux wins by being the coordination
   layer.

## Sources

- OpenCode GitHub: https://github.com/opencode-ai/opencode
- OpenCode (active): https://github.com/anomalyco/opencode
- OpenCode docs: https://opencode.ai/docs/
- OpenCode MCP docs: https://opencode.ai/docs/mcp-servers/
- OpenCode tools: https://opencode.ai/docs/tools/
- OpenCode agents: https://opencode.ai/docs/agents/
- OpenCode internals deep dive: https://cefboud.com/posts/coding-agents-internals-opencode-deepdive/
- Aider: https://github.com/Aider-AI/aider
- Cline: https://github.com/cline/cline
- Continue: https://github.com/continuedev/continue
- Goose: https://github.com/block/goose
- Cody: https://sourcegraph.com/cody
- Agentic AI Foundation: https://www.linuxfoundation.org/press/linux-foundation-announces-the-formation-of-the-agentic-ai-foundation
