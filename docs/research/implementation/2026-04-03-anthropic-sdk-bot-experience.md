# Anthropic SDK & Bot Experience Research

Research into building an excellent developer experience for an AI coding bot that participates in bpmux group chat via WebSocket, focused on the Anthropic ecosystem.

---

## 1. Anthropic SDK Options for Rust

There is no official Anthropic Rust SDK. All options are community-maintained. Here is a comparative assessment of what exists.

### anthropic-sdk-rust (tmikus)

- **Crate:** https://crates.io/crates/anthropic-sdk-rust
- **GitHub:** https://github.com/tmikus/anthropic-sdk-rust
- **Version:** 0.1.1
- **Stars:** ~0 (nascent)
- **Features:** Messages API, streaming, tool use, vision/multimodal, batch processing, files API, token counting, retry with exponential backoff, builder pattern, full async/tokio
- **Modules:** `client`, `resources` (messages, batches, files, models), `types`, `streaming`, `tools` (registry, execution, conversation management), `http`, `tokens`
- **Assessment:** Most feature-complete of the Rust options. Claims parity with the official TypeScript SDK. However, near-zero community adoption makes production readiness questionable. The architecture is sound (dedicated streaming module, tool registry, error types) but unproven.

### anthropic_rust

- **Crate:** https://crates.io/crates/anthropic_rust
- **Docs:** https://docs.rs/anthropic_rust/latest/anthropic_rust/
- **Version:** 0.1.3
- **Features:** Streaming (`MessageStream`, `StreamEvent`, `ContentDelta`), tool use (`Tool`, `ToolBuilder`), multimodal content, client configuration with middleware/retry, token counting
- **Assessment:** Clean API surface with dedicated streaming types. Slightly more mature in terms of type design. Still early-stage. MIT licensed.

### anthropic-rs (AbdelStark)

- **Crate:** https://crates.io/crates/anthropic (publishes as `anthropic`)
- **GitHub:** https://github.com/AbdelStark/anthropic-rs
- **Version:** 0.0.8
- **Stars:** 69
- **Features:** Messages API, streaming (via reqwest-eventsource), tool use, tool results, typed builders
- **Assessment:** Most community traction (69 stars, 14 forks). But only 62.67% documented and explicitly described as "work in progress." Streaming uses reqwest-eventsource directly, which is a reasonable dependency choice. Oldest and most validated of the options, but also lowest version number.

### async-anthropic (bosun-ai)

- **Crate:** https://crates.io/crates/async-anthropic
- **GitHub:** https://github.com/bosun-ai/async-anthropic
- **Version:** 0.6.0 (May 2025)
- **Stars:** 11
- **Features:** Messages API, Models API, tool use, streaming, automatic backoff, tracing
- **Assessment:** Most recent release activity. Built for bosun-ai's own production use, which gives it more real-world pressure than hobby projects. Acknowledges missing features. 73 commits across 7 releases suggests active iteration. Reasonable choice if you value a crate that someone is actually using.

### anthropic-sdk (Mixpeal)

- **GitHub:** https://github.com/Mixpeal/anthropic-sdk
- **Features:** Streaming and async support
- **Assessment:** Appears to be a fork or variant. Minimal information available.

### anthropic-ai-sdk

- **Crate:** https://crates.io/crates/anthropic-ai-sdk
- **Assessment:** Minimal documentation. Not enough information to evaluate.

### Recommendation

None of these are production-ready in the way that the official Python or TypeScript SDKs are. The realistic options are:

1. **Use async-anthropic** -- most actively maintained, built for real use, has tool support
2. **Use anthropic-rs** -- most community traction, oldest lineage
3. **Build a thin client directly** -- the Anthropic Messages API is a single HTTP endpoint with SSE streaming. A purpose-built client with reqwest + eventsource-stream might be more maintainable than depending on an immature third-party crate.

Option 3 is probably the right call for bpmux. The API surface is small (one endpoint, one streaming protocol). A thin client gives full control over error handling, retry logic, and streaming behavior -- all critical for a chat bot.

---

## 2. The Claude Agent SDK

### What It Is

Anthropic renamed the "Claude Code SDK" to the "Claude Agent SDK." It lets you use Claude Code's agent loop, tools, and context management as a library.

- **Docs:** https://platform.claude.com/docs/en/agent-sdk/overview
- **Python:** https://github.com/anthropics/claude-agent-sdk-python (pip install claude-agent-sdk)
- **TypeScript:** https://github.com/anthropics/claude-agent-sdk-typescript (npm install @anthropic-ai/claude-agent-sdk)
- **Rust:** No. Python and TypeScript only.

### What It Does

The Agent SDK provides:

- **Built-in tools:** Read, Write, Edit, Bash, Glob, Grep, WebSearch, WebFetch, AskUserQuestion
- **Autonomous tool loop:** You send a prompt, it executes tools until done -- you do not implement the tool execution loop yourself
- **Hooks:** PreToolUse, PostToolUse, Stop, SessionStart, SessionEnd callbacks
- **Subagents:** Spawn specialized child agents with restricted tool sets
- **MCP support:** Connect Model Context Protocol servers
- **Sessions:** Resume conversations with full context, fork sessions
- **Permissions:** Fine-grained control over which tools are available
- **CLAUDE.md integration:** Reads project instructions from filesystem

### Key Distinction from Client SDK

The Client SDK (anthropic Python/TS package) gives you raw API access -- you implement the tool loop. The Agent SDK gives you Claude Code's full agent loop as a library.

### Relevance to bpmux

The Agent SDK is relevant if the bpmux bot needs to be an autonomous coding agent (read files, run commands, edit code). It is NOT relevant if the bot is a conversational participant that discusses code without touching files.

Critical limitation: it is Python/TypeScript only. For a Rust application, you would either:

1. Shell out to a Python/TypeScript process running the Agent SDK
2. Implement the agent loop yourself against the Messages API
3. Use MCP to bridge capabilities

The Agent SDK establishes these patterns worth adopting regardless of language:
- Explicit tool allow-lists per conversation
- Hook-based lifecycle management
- Session persistence and resumption
- Subagent delegation for focused subtasks

---

## 3. Streaming Responses

### SSE Protocol for Anthropic Messages API

Source: https://platform.claude.com/docs/en/build-with-claude/streaming

Set `"stream": true` in the Messages API request. The response is server-sent events.

### Event Flow

1. `message_start` -- contains a `Message` object with empty `content`
2. For each content block:
   - `content_block_start`
   - One or more `content_block_delta` events
   - `content_block_stop`
3. `message_delta` -- top-level changes (usage counts are cumulative)
4. `message_stop`

Additional event types: `ping` (keepalive), `error` (e.g., overloaded_error).

### Content Block Delta Types

- **text_delta:** `{"type": "text_delta", "text": "ello frien"}` -- the actual text tokens
- **input_json_delta:** `{"type": "input_json_delta", "partial_json": "{\"location\": \"San Fra"}` -- for tool use, partial JSON accumulation
- **thinking_delta:** Extended thinking content (when enabled)
- **signature_delta:** Integrity signature for thinking blocks

### Implementation in Rust

For a Rust implementation, the key crates are:

- **reqwest** -- HTTP client, supports streaming response bodies
- **eventsource-stream** (https://crates.io/crates/eventsource-stream) -- SSE parser that converts a byte stream into typed events. 270K+ monthly downloads, used by 455 crates. Call `.eventsource()` on any byte stream to get an async Stream of SSE events.
- **eventsource-client** (https://crates.io/crates/eventsource-client, by LaunchDarkly) -- Higher-level SSE client with built-in HTTP transport (hyper v1), automatic reconnection, and redirect handling. More batteries-included than eventsource-stream.

### Sketch of the Rust Implementation

The pattern would be:

1. POST to `https://api.anthropic.com/v1/messages` with `"stream": true`
2. Get the response body as a byte stream
3. Pipe through eventsource-stream to parse SSE frames
4. Match on event types: `message_start`, `content_block_delta`, `message_stop`, etc.
5. For `text_delta` events, extract the `text` field and push it to the WebSocket as a partial message
6. For `content_block_stop`, finalize the message

The key design decision for bpmux: how to relay streaming tokens to the group chat. Options:
- **Token-by-token:** Send each delta as a WebSocket message. Most responsive but chatty.
- **Chunked:** Buffer tokens and send every ~100ms or every N tokens. Good balance.
- **Line-by-line:** Buffer until a newline, then send. Natural for code output.

---

## 4. System Prompts for Coding Assistants

### Claude Code's Prompt Architecture

Source: https://github.com/Piebald-AI/claude-code-system-prompts

Claude Code does NOT use a single monolithic prompt. It uses a modular, conditional architecture with 110+ separate strings:

- **Conditional components** dynamically added based on environment, configuration, and mode
- **Tool descriptions** as individual system context blocks (24 built-in tools)
- **Specialized agent prompts** for Plan, Explore, and Task subagents
- **Behavioral reminders** (~40 system reminders for file states, hooks, IDE interactions)
- **Mode-specific instructions** (plan mode, learning mode, auto mode, minimal mode, buddy mode)

Key patterns:
- Each component updates independently across releases
- Context-specific customization without maintaining one giant prompt
- Security instructions are embedded throughout

### Aider's Prompt Design

Source: https://github.com/Aider-AI/aider/blob/main/aider/coders/editblock_prompts.py

Aider's approach is more focused:
- Instructs the AI to "act as an expert software developer"
- Emphasizes respecting existing code conventions and libraries
- Defines a strict SEARCH/REPLACE block format for file edits
- Requires explicit file context before editing (user must add files to chat)
- Asks for step-by-step explanations before changes
- CONVENTIONS.md file forwarded to the LLM as project standards

Key insight: Aider keeps the prompt simple and procedural. The format is the prompt -- the SEARCH/REPLACE block structure constrains the AI's behavior more than instructions alone would.

### OpenCode's Prompt Architecture

Sources:
- https://github.com/opencode-ai/opencode
- https://cefboud.com/posts/coding-agents-internals-opencode-deepdive/

OpenCode uses a layered, provider-aware approach:
- **Provider-specific prompts** (Anthropic, OpenAI, Gemini each get customized versions)
- **Environmental context** (working directory, platform, date)
- **Agent-specific instructions** (plan agent cannot edit files)
- **Automatic summarization** when tokens exceed 90% of context limit
- **LSP integration** provides real-time code diagnostics
- **Git snapshots** at each step for rollback

Design principle: the system prompt is the agent's identity and constraints; the tools are the agent's capabilities. Separation of concerns.

### Patterns That Produce Good Coding Assistance

Across all three systems, common patterns emerge:

1. **Role definition is brief** -- "Act as an expert software developer" or equivalent. A sentence, not a paragraph.
2. **Behavioral constraints are specific** -- "Only return code in SEARCH/REPLACE blocks" is more effective than "be careful with code."
3. **Project context is injected, not hardcoded** -- CLAUDE.md, CONVENTIONS.md, AGENTS.md files let each project customize behavior.
4. **Tool descriptions double as behavior instructions** -- how you describe a tool shapes how the AI uses it.
5. **Security is threaded throughout, not appended** -- safety instructions appear at the point of use, not in a single block at the end.
6. **Modes change the prompt, not the conversation** -- different modes (plan, explore, execute) swap out system prompt sections.

### For bpmux's Bot

A group chat coding bot needs a different prompt than a solo coding agent. Key differences:
- Must understand it is one participant among several (humans and possibly other AIs)
- Must not dominate the conversation -- should wait to be addressed or respond to relevant messages
- Should explain reasoning conversationally, not in tool-output format
- Should reference what others have said (conversation threading)
- Should be transparent about uncertainty and limitations

---

## 5. Tool Use vs. Pure Conversation

### Can a Bot Be Useful WITHOUT Tools?

Yes, but with significant limitations. A tool-less bot can:

- **Review code** that is pasted into chat
- **Explain concepts** and design patterns
- **Suggest approaches** to problems described in chat
- **Spot bugs** in code snippets shared in messages
- **Discuss architecture** at a conceptual level
- **Answer questions** about languages, frameworks, APIs
- **Rubber duck** -- ask clarifying questions that help developers think

A tool-less bot CANNOT:

- Read the actual codebase
- Verify its suggestions compile or pass tests
- Make changes
- See the current state of the project
- Provide accurate answers about project-specific patterns

### The Navigator in Pair Programming

Sources:
- https://stackoverflow.blog/2024/04/03/developers-with-ai-assistants-need-to-follow-the-pair-programming-model/
- https://graphite.com/guides/ai-pair-programming-best-practices

In pair programming, the navigator:
- Directs overall strategy
- Makes architectural decisions
- Reviews code the driver produces
- Catches errors before they become code
- Thinks about what's next while the driver focuses on now

An AI CAN be a navigator without tools if someone else (a human or another AI) is the driver. The bot would:
- Review code diffs pasted in chat
- Suggest what to work on next
- Identify potential issues in described approaches
- Ask questions that reveal hidden assumptions

This is genuinely useful. The question is whether it is useful *enough* to justify building a bot.

### The Minimum Viable Tool Set

If the bot DOES get tools, the minimum viable set from looking at Claude Code, Aider, and OpenCode:

| Tool | Purpose | Why Essential |
|------|---------|--------------|
| **Read file** | See code in the project | Cannot give project-specific advice without it |
| **Grep/search** | Find patterns across codebase | Contextualizes questions |
| **List files/glob** | Understand project structure | Orients the bot |

That is the read-only set. It makes the bot dramatically more useful without any write risk.

For write access, the minimum adds:

| Tool | Purpose |
|------|---------|
| **Edit file** | Make targeted changes |
| **Run command** | Execute tests, builds, lints |

### Recommendation for bpmux

Start with a conversational navigator (no tools). This is:
- Simpler to build
- Safer (no file system access)
- Still useful for code review, architecture discussion, and rubber ducking
- A forcing function to get the chat protocol right before adding complexity

Add read-only tools (file read, grep, glob) as the first upgrade. This is where the bot becomes genuinely compelling -- it can read the code being discussed and give specific, grounded feedback.

Add write tools only when there is a clear use case and a permission model to support it.

---

## 6. Context Management for a Chat Bot

### The Problem

LLMs have finite context windows. A group chat generates messages continuously. You cannot send the entire chat history to the API on every request.

### Strategies

Sources:
- https://apxml.com/courses/langchain-production-llm/chapter-3-advanced-memory-management/context-window-management
- https://agenta.ai/blog/top-6-techniques-to-manage-context-length-in-llms
- https://mem0.ai/blog/llm-chat-history-summarization-guide-2025
- https://community.openai.com/t/best-practices-for-cost-efficient-high-quality-context-management-in-long-ai-chats/1373996

#### 1. Sliding Window

Keep the last N messages. Discard older ones.
- **Pro:** Simple, predictable token usage
- **Con:** Loses important context from earlier in the conversation
- **Best for:** Casual chat bots where earlier context is rarely referenced

#### 2. Sliding Window + Summary

Keep last N messages verbatim. Summarize older messages into a running summary.
- **Pro:** Retains key information from earlier conversation
- **Con:** Summarization costs tokens and can lose nuance; requires an LLM call to summarize
- **Best for:** Long-running conversations where early context matters

#### 3. Token Budget with Prioritization

Allocate a fixed token budget. Fill it with the most relevant messages, not just the most recent.
- **Pro:** Maximizes context quality
- **Con:** Requires relevance scoring, which is hard
- **Best for:** Bot that needs to respond to specific questions about past discussion

#### 4. Hierarchical Memory

Short-term: last N messages. Medium-term: summarized recent conversation. Long-term: extracted facts and decisions.
- **Pro:** Models how human memory works
- **Con:** Complex to implement; long-term extraction is error-prone
- **Best for:** Agent systems with persistent state

### How Other Bots Handle This

- **Discord LLM bots:** Typically use sliding window of 10-20 messages. Some use ConversationTokenBufferMemory from LangChain with a max_token_limit.
- **Slack LLM bots:** Similar sliding window approach. The Vokturz/LLM-slackbot-channels project on GitHub uses max token limits analogous to ConversationTokenBufferMemory.
- **OpenCode:** Automatically summarizes when tokens exceed 90% of context limit. Summary focuses on accomplishments, modified files, and next steps.
- **Claude Code:** Uses conversation compaction -- a specialized summarization that preserves tool results and file context.

### Recommendation for bpmux

For a group chat coding bot:

1. **System prompt + project context:** Fixed, cached (see section 7)
2. **Recent messages:** Sliding window of last ~50 messages (the bot sees the chat)
3. **Bot's own recent exchanges:** Keep the bot's recent responses and the messages that triggered them (higher priority than other chat messages)
4. **Summarization trigger:** When the sliding window exceeds 80% of remaining context budget, summarize the oldest half into a paragraph

The key insight: in a group chat, not every message is directed at the bot. The context manager should distinguish between:
- Messages addressed to the bot (high priority)
- Messages about topics the bot has been discussing (medium priority)
- General chat between humans (low priority, summarize or drop first)

---

## 7. Prompt Caching

### How Anthropic's Prompt Caching Works

Source: https://platform.claude.com/docs/en/build-with-claude/prompt-caching

### Pricing

| Component | Cost vs. Base Input |
|-----------|-------------------|
| Cache write (5-min TTL) | 1.25x |
| Cache write (1-hour TTL) | 2.0x |
| Cache read | 0.1x |

For Claude Sonnet 4, base input is $3/MTok:
- 5-min cache write: $3.75/MTok
- 1-hour cache write: $6/MTok
- Cache read: $0.30/MTok

This is a 90% reduction on cache reads.

### Minimum Cacheable Sizes

| Model | Minimum Tokens |
|-------|---------------|
| Claude Opus 4.6, 4.5 | 4,096 |
| Claude Sonnet 4.6 | 2,048 |
| Claude Sonnet 4.5, 4, 3.7 | 1,024 |
| Claude Haiku 4.5 | 4,096 |
| Claude Haiku 3.5, 3 | 2,048 |

### Two Caching Modes

**Automatic caching** (recommended for chat): Set `cache_control` at the top level. The cache breakpoint moves forward automatically as the conversation grows. Each new request caches everything up to the last cacheable block. Previous turns are read from cache.

**Explicit breakpoints:** Place `cache_control` on individual content blocks. Up to 4 breakpoints. Gives fine-grained control.

### Multi-Turn Conversation Pattern

With automatic caching:

```
Request 1: [System + User1]                    -> all written to cache
Request 2: [System + User1 + Asst1 + User2]    -> System+User1 read from cache
Request 3: [System + ... + Asst2 + User3]       -> everything through User2 read from cache
```

The cache grows incrementally. Each request only pays full price for the new tokens since the last cache point.

### Implementation Pattern for bpmux Bot

The bot's API calls have a natural layered structure:

```
Layer 1 (stable, cache with 1-hour TTL):
  - System prompt
  - Tool definitions (if any)
  - Project context (CLAUDE.md equivalent)

Layer 2 (semi-stable, cache with 5-min TTL):
  - Conversation summary (updated periodically)
  - Recent chat history (grows between summarizations)

Layer 3 (uncached):
  - The current message being responded to
```

### Cache Hierarchy Rules

Anthropic caches in this order: `tools` -> `system` -> `messages`. Changes at any level invalidate that level and everything after it. This means:

- Tool definitions should be stable (put them first, cache them)
- System prompt should be stable (cache it)
- Messages change every turn (the growing conversation)

If you change the system prompt, ALL cache is invalidated. So:
- Keep the system prompt truly static
- Put dynamic project context in the messages, not the system prompt
- Or split the system prompt into static + dynamic blocks with separate cache breakpoints

### Cost Impact Estimate

For a bot that responds to ~100 messages per day with a 4K token system prompt and ~10K tokens of cached conversation context:

- Without caching: ~14K input tokens per request x 100 = 1.4M input tokens/day
- With caching: ~4K fresh tokens + ~10K at 0.1x = ~5K effective tokens per request = 500K effective tokens/day
- Savings: ~64% on input token costs

With a long system prompt (8K+ tokens) and significant cached conversation, savings approach 90%.

---

## Summary of Key Decisions

| Decision | Recommendation | Confidence |
|----------|---------------|------------|
| Rust SDK | Build thin client with reqwest + eventsource-stream | High |
| Agent SDK | Not directly usable (no Rust), but adopt its patterns | High |
| Streaming | Chunked delivery (~100ms batches) over WebSocket | Medium |
| System prompt | Modular, with project context injection via CLAUDE.md pattern | High |
| Initial tool set | None (conversational navigator first) | Medium |
| Context management | Sliding window + periodic summarization + message priority | High |
| Prompt caching | Automatic caching with layered content structure | High |

### Sources Referenced

- Anthropic Messages API Streaming: https://platform.claude.com/docs/en/build-with-claude/streaming
- Anthropic Prompt Caching: https://platform.claude.com/docs/en/build-with-claude/prompt-caching
- Claude Agent SDK: https://platform.claude.com/docs/en/agent-sdk/overview
- Claude Code System Prompts: https://github.com/Piebald-AI/claude-code-system-prompts
- Aider Prompts: https://github.com/Aider-AI/aider/blob/main/aider/coders/editblock_prompts.py
- OpenCode Internals: https://cefboud.com/posts/coding-agents-internals-opencode-deepdive/
- OpenCode Docs: https://opencode.ai/docs/
- anthropic-sdk-rust: https://github.com/tmikus/anthropic-sdk-rust
- anthropic_rust: https://crates.io/crates/anthropic_rust
- anthropic-rs: https://github.com/AbdelStark/anthropic-rs
- async-anthropic: https://github.com/bosun-ai/async-anthropic
- eventsource-stream: https://crates.io/crates/eventsource-stream
- eventsource-client: https://crates.io/crates/eventsource-client
- Prompt Caching Cost Study: https://yuv.ai/blog/prompt-caching-cut-our-ai-costs-by-70
- Context Window Management: https://agenta.ai/blog/top-6-techniques-to-manage-context-length-in-llms
- LLM Chat History Summarization: https://mem0.ai/blog/llm-chat-history-summarization-guide-2025
- AI Pair Programming Model: https://stackoverflow.blog/2024/04/03/developers-with-ai-assistants-need-to-follow-the-pair-programming-model/
- Pair Programming Best Practices: https://graphite.com/guides/ai-pair-programming-best-practices
- System Prompts Collection: https://github.com/x1xhlol/system-prompts-and-models-of-ai-tools
