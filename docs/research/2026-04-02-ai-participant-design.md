# AI Participant Design: Making the AI a Real Conversation Member

Date: 2026-04-02
Status: Proposal (synthesized from seven parallel research agents)

## The Problem

MCP is request/response. The AI can only see messages when it actively calls `read_messages`. It cannot receive messages in real time. It cannot spontaneously respond. bpmux needs the AI to be a real participant -- receiving messages as they arrive and responding naturally, like a human in a group chat.

The current implementation has bpmux as a passive message store that AIs poll. This works for a tool, but not for a participant.

## What the Research Found

Seven agents investigated different angles. Here are the key findings that drive the design.

### Finding 1: Every AI chat tool uses the same loop

Aider, Claude Code, OpenCode, Codex CLI, Continue.dev -- they all use the same fundamental pattern:

```
loop {
    collect_messages()
    response = llm.send(full_history)
    handle_response(response)  // may include tool calls
}
```

The AI is stateless. It has no persistent process watching for messages. The conversation history IS the AI's memory -- the full message list is sent with every API call. The application decides when to call the AI and what context to provide.

This is not a limitation to work around. It is the architecture.

### Finding 2: The Claude Agent SDK is powerful but wrong for bpmux

The Agent SDK (Python/TypeScript) provides Claude Code's full agent loop: tools, streaming, context management, subagents, hooks. It is production-grade infrastructure.

But it is wrong for bpmux because:
- **Language mismatch**: Python/TypeScript only. bpmux is Rust. Would require a sidecar process.
- **Claude-only**: Ties bpmux to one provider. bpmux's value is provider-agnostic group chat.
- **Task-oriented**: The SDK runs a task to completion. bpmux needs an ongoing conversation participant, not a one-shot task runner.
- **Overkill**: The SDK provides file ops, shell execution, search, subagents. bpmux does not need coding tools -- it is the communication layer. Participants bring their own tools.

### Finding 3: The OpenAI Chat Completions API is the universal interface

Nearly every LLM provider exposes an OpenAI-compatible endpoint: Ollama, vLLM, Together AI, Groq, Fireworks, LM Studio, and many more. The shape is always:

```
POST /v1/chat/completions
{model, messages: [{role, content}], tools?, stream?, temperature?, max_tokens?}
```

Anthropic's Messages API is slightly different in field names but identical in concept. If bpmux implements OpenAI-compat + an Anthropic adapter, it covers the vast majority of providers.

### Finding 4: A2A protocol is closer than MCP but still mismatched

Google's Agent-to-Agent (A2A) protocol supports push notifications (webhooks), SSE streaming, and multi-turn conversations via `input-required` states. It is better than MCP for real-time interaction.

But it is task-oriented, not conversation-oriented. An AI agent cannot spontaneously contribute -- it can only respond within the scope of an assigned task. There is no group/broadcast primitive. bpmux would be building a chat protocol on top of a task protocol.

Rust implementations exist (a2a-rs is the most mature), but A2A adds protocol complexity without solving the fundamental problem.

### Finding 5: The "AI sees every message" illusion

No existing tool has the AI "in" the conversation as a peer. In every case:

1. The AI is stateless -- no persistent process watching messages
2. The conversation history IS the AI's memory
3. The loop owner is the application

The AI "sees every message" not because it is listening, but because the application replays the entire conversation to it on each turn. This is a feature, not a bug -- it means bpmux controls exactly what context the AI has.

### Finding 6: Direct API calls are simpler and sufficient

Claude Code itself is just a sophisticated Messages API client with a message queue, tool executor, context compactor, and TUI. There is no special API -- it uses the same public Messages API available to everyone.

For bpmux, the hardest parts of going direct are:
- **Context window management** (token counting + compaction)
- **Streaming SSE consumer** in Rust

Everything else is straightforward HTTP + JSON. The `reqwest` + `serde` + `eventsource-stream` stack handles it.

### Finding 7: Provider abstraction is a solved pattern

OpenCode demonstrates a clean two-layer abstraction in Go that maps directly to Rust:
- A `Provider` trait with `send()` and `stream()` methods
- Per-provider implementations (Anthropic, OpenAI-compat, etc.)
- Normalized message/event types that hide provider differences

The Rust ecosystem has options: the `llm` crate (graniet), `rig`, `async-anthropic`, `anthropic-sdk-rust`. A hand-rolled thin client (~300 lines) using `reqwest` + `serde` is also viable given the small API surface.

## Recommended Architecture

### The Core Idea: bpmux Owns the Conversation Loop

bpmux becomes the agent loop owner. Instead of being a passive message store that AIs poll via MCP, bpmux actively manages AI participants:

1. When a message arrives (from any source: human TUI, peer WebRTC, MCP client), bpmux appends it to the log
2. bpmux decides whether to invoke the AI (trigger policy -- see below)
3. bpmux assembles the context (message history, system prompt) and calls the LLM API directly
4. bpmux streams the response into the chat as a first-class message
5. The AI's response is visible to all participants immediately

The AI becomes a participant because bpmux makes it one -- not because the AI has a persistent connection.

```
                    ┌────────────────────────┐
                    │     bpmux node         │
                    │                        │
  Human ──TUI────►  │  Message Log (JSONL)   │ ◄────WebRTC──── Peer
                    │         │              │
  MCP Client ────►  │         ▼              │
                    │  Trigger Policy        │
                    │  "Should the AI speak?"│
                    │         │              │
                    │         ▼              │
                    │  LLM Client            │
                    │  (direct API call)     │
                    │         │              │
                    │         ▼              │
                    │  AI Response → Log     │
                    │  (streamed, visible)   │
                    └────────────────────────┘
```

### What This Changes from the Current Design

**Keeps:**
- TUI (ratatui + crossterm) -- unchanged
- Session log (JSONL) -- unchanged, remains the source of truth
- WebRTC plans -- unchanged, peer messages still flow P2P
- Web UI plans -- unchanged
- MCP server -- still runs for external AI tools that want to participate the old way
- Message model -- unchanged
- Token awareness / read cursors -- still valuable for MCP clients

**Adds:**
- `src/ai/` -- new bounded context for AI participant management
  - `provider.rs` -- `AiProvider` trait with `send()` and `stream()` methods
  - `anthropic.rs` -- Anthropic Messages API implementation
  - `openai_compat.rs` -- OpenAI-compatible endpoint implementation (covers Ollama, cloud providers)
  - `context.rs` -- context window management (token counting, history assembly, compaction)
  - `trigger.rs` -- trigger policy (when does the AI respond?)
  - `participant.rs` -- orchestrates the AI as a conversation member
- Configuration for AI providers in `~/.bpmux/config.toml`

**Changes:**
- The AI participant is now a first-class concept in bpmux, not an external tool connecting via MCP
- bpmux manages the AI's context window, not the external tool
- The system prompt is bpmux's responsibility -- it defines the AI's role in the conversation

**Does NOT remove:**
- MCP server stays. External AI tools (Claude Code, OpenCode) can still connect as MCP clients. This is the "bring your own agent" path. The new AI participant is the "bpmux-managed AI" path. Both coexist.

### Trigger Policy: When Does the AI Respond?

This is the design question with the most uncertainty. Options:

**Option A: Respond to every message (simple, expensive)**
Every non-AI message triggers an AI response. Simple but creates a chatty loop and burns tokens.

**Option B: Respond when addressed (explicit, safe)**
The AI responds only when mentioned by name or tagged (e.g., `@claude`, `/ask`). This is the safest default -- no surprise token usage. The human controls when the AI speaks.

**Option C: Respond on idle (conversational)**
The AI responds after a configurable silence period (e.g., 3 seconds with no new messages). This feels more like a natural conversation participant -- it waits for a pause before speaking. Risk: the AI may respond when not wanted.

**Option D: Hybrid (recommended)**
- Default: respond when addressed (`@claude` or `/ask`)
- Configurable: auto-respond mode where the AI responds after a silence period
- Always: respond when another AI sends a message directed to it
- Never respond to its own messages (prevent infinite loops)

The trigger policy is a configuration concern, not an architectural one. The system should support all modes and let the user choose.

### Provider Abstraction

The minimal trait:

```rust
#[async_trait]
trait AiProvider: Send + Sync {
    /// Send messages and get a complete response
    async fn send(&self, request: &CompletionRequest) -> Result<CompletionResponse>;

    /// Send messages and stream the response token by token
    async fn stream(&self, request: &CompletionRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>>>>>;
}

struct CompletionRequest {
    system_prompt: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

struct ChatMessage {
    role: Role,       // User, Assistant, System
    content: String,
    name: Option<String>,  // participant identity
}

struct CompletionResponse {
    content: String,
    usage: TokenUsage,
    finish_reason: FinishReason,
}

struct StreamChunk {
    delta: String,       // partial text
    usage: Option<TokenUsage>,
}
```

This is the OpenAI Chat Completions shape. Anthropic's Messages API maps to it with straightforward field translation.

### Context Management

The AI's context window is finite. bpmux must decide what to include:

1. **System prompt** -- AI's role, conversation rules, project context (from CLAUDE.md or equivalent)
2. **Recent messages** -- sliding window of the most recent N messages
3. **Summary** -- optional compressed summary of older messages

**Strategy for v1:** Start with a simple sliding window. Include the last N messages (configurable, default 50) in each API call. Track token usage from API responses. When approaching the context limit, truncate from the oldest messages.

**Future:** Add summarization (call the AI to summarize older messages) and persistent memory formation. These are well-understood patterns but add complexity.

### System Prompt

The system prompt defines the AI's role in the conversation. This is bpmux's responsibility, not the user's:

```
You are a participant in a group programming session via bpmux.
Other participants: {participant_list}
Project context: {from CLAUDE.md or config}

Guidelines:
- You are one voice in a conversation, not the sole assistant
- Be concise -- this is chat, not a report
- When you don't know, say so
- Reference specific messages by quoting them
- If multiple participants are discussing, wait for a natural pause
```

The system prompt is assembled fresh on every API call (since the API is stateless). It can incorporate live context (current participants, connection status, recent activity summary).

## Multi-Provider Support

### Day 1: Anthropic (Claude)

bpmux is being built alongside Claude Code. Claude is the first provider. The Anthropic Messages API is well-documented, has community Rust clients, and supports streaming.

Implementation: Direct HTTP calls via `reqwest`, SSE parsing via `eventsource-stream`, JSON via `serde`. Approximately 300-500 lines of Rust for a working client.

### Day 2: OpenAI-Compatible (Ollama, cloud providers)

The OpenAI Chat Completions API is the universal interface. One implementation covers Ollama (local models), vLLM, Together AI, Groq, Fireworks, LM Studio, and more. The only difference is the base URL and API key.

Implementation: Same HTTP stack, different request/response shapes. Another 200-300 lines.

### Day 3+: Additional providers

The `AiProvider` trait makes adding providers straightforward. Each provider is a struct that implements `send()` and `stream()` with provider-specific HTTP calls and response parsing.

### Configuration

```toml
[ai]
# Provider: "anthropic", "openai-compat", "ollama"
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"  # env var name, not the key itself

# For openai-compat / ollama:
# base_url = "http://localhost:11434/v1"
# model = "llama3.1"
# api_key_env = ""  # ollama needs no key

[ai.trigger]
# "addressed" = respond when @mentioned
# "auto" = respond after silence_ms of quiet
# "off" = never respond (MCP-only mode)
mode = "addressed"
silence_ms = 3000
```

## The Minimal First Step

The thinnest slice that makes the AI a real participant:

### Slice: "Human types, AI responds in the chat"

1. Add `src/ai/provider.rs` with the `AiProvider` trait
2. Add `src/ai/anthropic.rs` implementing Anthropic Messages API (streaming)
3. Add `src/ai/participant.rs` that watches the message log and calls the provider
4. Wire it into `lib.rs`: when the human sends a message containing `@ai` (or similar trigger), bpmux assembles the recent message history and calls the API
5. Stream the response into the chat log as `<name>:ai` messages
6. The response appears in the TUI in real time, character by character

**What this proves:**
- The AI is a conversation participant, not a polled tool
- The architecture works end-to-end (message -> trigger -> API call -> streamed response -> log -> TUI)
- Multi-provider is possible (the trait is in place even if only Anthropic is implemented)

**What this defers:**
- OpenAI-compat provider (Ollama, etc.)
- Sophisticated trigger policies (auto-respond, silence detection)
- Context compaction / summarization
- Tool calling (the AI just chats, no code execution)
- WebRTC integration (peer messages triggering AI responses)

**Estimated scope:** ~500-800 lines of new Rust code across 3-4 files.

## Risks and Tradeoffs

### Risk 1: Cost

**Real.** Every AI response costs tokens. With direct API calls, bpmux users pay per-call. A chatty trigger policy could burn through budgets quickly.

**Mitigation:** Default to "addressed" trigger mode. Display cost estimates in the TUI. Set a per-session budget cap. Use Sonnet (cheaper) as default, not Opus.

**Unknown:** How much does a typical pair programming conversation cost? We need to measure this in practice. The existing token awareness design (budgets, read cursors) helps, but now applies to outbound API calls too, not just inbound message windows.

### Risk 2: Latency

**Real.** API calls take 1-5 seconds for the first token, more for long responses. The conversation will feel different from human-speed chat.

**Mitigation:** Streaming. Show tokens as they arrive. Display a "thinking..." indicator while waiting for the first token. This is what every AI chat tool does and users accept it.

### Risk 3: Two paths for AI participation

The design creates two ways an AI can participate:
1. **bpmux-managed**: bpmux calls the LLM API directly (new)
2. **MCP client**: An external AI tool connects via MCP and polls (existing)

These could confuse users or create inconsistent behavior.

**Mitigation:** Frame them clearly. MCP is "bring your own agent" -- for users who already have Claude Code or similar tools running. bpmux-managed AI is "built-in assistant" -- for users who want a participant without external tooling. They serve different use cases and can coexist.

### Risk 4: Context window management is hard

**Real.** This is the most complex part of building an LLM client. Token counting, summarization, compaction -- Claude Code has thousands of lines dedicated to this.

**Mitigation:** Start simple. A sliding window of recent messages with a configurable size. No summarization in v1. The message log is the source of truth -- the AI always sees a bounded window of it. Add compaction/summarization as a later optimization when we have real usage data.

### Risk 5: Provider abstraction may leak

Different providers have different capabilities: tool calling quality varies, streaming formats differ slightly, some support extended thinking, others don't. The abstraction may not hold cleanly.

**Mitigation:** Keep the trait minimal. Only abstract what all providers share: send messages, get text back, stream chunks. Provider-specific features (extended thinking, tool calling) stay behind provider-specific configuration, not in the trait.

### Risk 6: The AI might not be a good group chat participant

**Hypothesis.** We assume an AI can participate naturally in a multi-party programming conversation. This is largely unproven. Most AI chat is 1:1 (one human, one AI). Group dynamics are different.

**Mitigation:** This is the core hypothesis bpmux exists to test. The trigger policy and system prompt are the levers. Start conservative (addressed-only), observe behavior, iterate. The architecture supports changing the policy without changing the plumbing.

## What is Proven vs What is Hypothesis

### Proven

- Direct API calls to LLM providers work (every AI chat tool does this)
- Streaming SSE works for real-time token display (universal pattern)
- The OpenAI Chat Completions API is a viable universal interface (industry convergence)
- Provider abstraction in Rust is feasible (multiple crates exist, OpenCode demonstrates the pattern in Go)
- Conversation history replay is how AI "remembers" (every tool does this)
- The REPL loop (collect messages -> call API -> display response) is the fundamental pattern

### Hypothesis (needs validation)

- An AI can participate naturally in a multi-party group chat (not just 1:1)
- The "addressed" trigger policy feels natural (vs auto-respond vs manual invocation)
- A sliding window of 50 messages provides sufficient context for useful AI responses
- The cost of direct API calls is acceptable for pair programming use cases
- Users want a bpmux-managed AI participant vs just connecting their existing AI tool via MCP
- The system prompt can make the AI behave as a conversation peer rather than a question-answering assistant

## Decision Summary

| Question | Answer | Confidence |
|---|---|---|
| Architecture? | Direct API calls from bpmux to LLM providers | High |
| Agent SDK? | No -- wrong language, wrong model, Claude-only | High |
| A2A protocol? | No -- task-oriented, adds complexity without solving the core problem | High |
| MCP for AI participation? | Keep for external tools, but not the primary AI participation mechanism | High |
| How does the AI see messages? | History replay on each API call (same as every other tool) | High |
| When does the AI respond? | Trigger policy (addressed by default, configurable) | Medium |
| Multi-provider? | AiProvider trait + per-provider implementations | High |
| First step? | Anthropic provider + addressed trigger + streaming response | High |
| Context management? | Sliding window, simple truncation, defer compaction | Medium |

## Appendix: Research Sources

1. **Claude Agent SDK** -- Python/TypeScript SDK with full agent loop, tools, hooks, streaming, session management. Powerful but Claude-only and not Rust.
2. **AI-agnostic patterns** -- Industry convergence on OpenAI Chat Completions as universal interface. The `llm` Rust crate covers 10+ providers. Sliding window + summarization for context management.
3. **OpenCode architecture** -- Two-layer provider abstraction (Provider wrapping ProviderClient). Polymorphic ContentPart enum. Agent loop as async task. Pub/sub for decoupling agent from TUI. Auto-compaction at 95% context usage. Patterns map cleanly to Rust.
4. **A2A protocol** -- Push notifications and SSE streaming, better than MCP for real-time. But task-oriented, not conversation-oriented. No group/broadcast primitive. Rust implementations exist (a2a-rs).
5. **How other tools solve real-time AI chat** -- Universal REPL pattern across all tools. AI is stateless; history is replayed. SSE streaming is standard. MUCA (Multi-User Chat Assistant) research shows multi-party AI chat is possible but understudied.
6. **Anthropic Messages API** -- Stateless, send full history each call. SSE streaming, tool use, prompt caching (90% savings on cached tokens). No official Rust SDK but community options exist. Claude Code itself is just a Messages API client.
7. **What bpmux needs as a coding agent** -- Three tools is the minimum (read, edit, shell). But bpmux should NOT build coding tools -- it is the communication layer. Option B (MCP server for external tools) is the right v1 path. Option A (Agent SDK sidecar) is a possible v2 path for built-in coding capability.
