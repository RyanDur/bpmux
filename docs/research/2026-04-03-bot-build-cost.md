# The Actual Cost of Building and Maintaining a Bot Inside bpmux

An honest accounting of what it takes to build an LLM bot in Rust, at three levels of ambition, with maintenance projections and opportunity cost analysis.

---

## 1. The Minimum Bot

**What it does:** System prompt + conversation window + one API call + streaming response. Reads the bpmux message log, sends it to Claude, streams the response back into the chat.

### Line Count Estimate

| Component | Lines of Rust | Notes |
|---|---|---|
| Anthropic HTTP client (reqwest + SSE parsing) | ~150 | POST to /v1/messages, parse SSE events, extract text deltas |
| Conversation window management | ~80 | Sliding window of last N messages, token budgeting |
| System prompt + message formatting | ~50 | Build the messages array, inject system prompt |
| WebSocket bot client | ~120 | Connect to bpmux, identify, receive messages, send responses |
| Bot orchestration (when to respond, loop prevention) | ~80 | Filter @mentions, ignore own messages, debounce |
| Streaming relay (tokens -> WebSocket messages) | ~60 | Buffer chunks, send partial messages or token-by-token |
| Error handling (timeouts, API failures, reconnection) | ~80 | Retry logic, partial response preservation, rate limit backoff |
| Configuration (API key, model, system prompt) | ~40 | Env vars, CLI args |
| Types and serialization | ~60 | Request/response structs, SSE event types |
| **Total** | **~720** | |

With tests (unit tests for message formatting, window management, SSE parsing): add ~300 lines. **Total with tests: ~1,020 lines.**

For context, the entire bpmux codebase today is ~1,050 lines of Rust (src/ only). The minimum bot roughly doubles the codebase.

### New Dependencies

| Crate | Purpose | Already in Cargo.toml? |
|---|---|---|
| reqwest | HTTP client for Anthropic API | No (only in dev-dependencies) |
| eventsource-stream | SSE parser for streaming responses | No |
| reqwest-eventsource | Bridges reqwest response to SSE stream | No |

That is 3 new production dependencies. reqwest is heavy (pulls in hyper, http, tower, rustls or native-tls). eventsource-stream is small. The dependency tree grows meaningfully.

### What Breaks When Anthropic Changes Their API

The Anthropic Messages API has been stable since the `2023-06-01` version header. But:

- **Model IDs change.** `claude-sonnet-4-20250514` will be superseded. The bot needs a config field for this, not a hardcoded string.
- **New features you need to adopt.** The migration from Claude 3.5 to 4.5 required updating tool version strings and removing beta headers. Cache control parameters changed between the 5-min and 1-hour TTL tiers.
- **Context window changes.** Anthropic is retiring the 1M context window beta for Sonnet 4.5/4 on April 30, 2026. If you built against that, you need to adapt.
- **Pricing changes.** Not a code change, but affects system prompt sizing and context window strategy. When cache read pricing dropped to 0.1x, caching strategies that were not worth implementing suddenly were.
- **SSE event types expand.** New delta types (thinking_delta, signature_delta) have been added over time. A parser that chokes on unknown event types breaks.

**Realistic maintenance cadence:** Touch the API client code 2-4 times per year. Each touch is small (swap a model ID, handle a new event type) but requires testing.

### Maintenance Burden Rating: Low-Medium

The API client is the most maintenance-sensitive piece. Everything else (WebSocket client, message formatting, conversation window) is stable once built. Budget 1-2 days per quarter for API-related updates.

---

## 2. The "Good Enough" Bot

Everything from the minimum bot, plus read-only tools: file read, grep/search, glob/find, git log/diff/status.

### Additional Code

| Component | Lines of Rust | Notes |
|---|---|---|
| Tool use protocol (tool calls + tool results in API) | ~150 | Parse tool_use content blocks, format tool_result messages, handle multi-turn tool loops |
| File read tool | ~80 | Read file with line numbers, handle binary detection, size limits |
| Grep/search tool | ~120 | Regex search across files, respect .gitignore, format results |
| Glob/find tool | ~60 | Pattern matching, directory traversal, result formatting |
| Git operations (log, diff, status, blame) | ~200 | Shell out to git or use gix crate, parse output, format for LLM |
| Tool dispatch and schema definitions | ~100 | JSON schemas for each tool, dispatch by name, validation |
| Agentic loop (tool call -> execute -> feed back -> repeat) | ~150 | The LLM calls a tool, you execute it, send the result back, repeat until the LLM stops calling tools |
| Context management for tool results | ~80 | Tool results are big. Truncation, summarization, token counting |
| **Additional lines** | **~940** | |
| **Running total (with minimum bot)** | **~1,660** | |

With tests: add ~500 lines (tool execution tests, mock API responses, integration tests for the agentic loop). **Total with tests: ~2,460 lines.**

### New Dependencies

| Crate | Purpose |
|---|---|
| ignore | .gitignore-aware directory walking (BurntSushi, used by ripgrep) |
| regex | Pattern matching for grep tool |
| gix (or shell out to git) | Git operations |
| globset or glob | File pattern matching |

If you use gix: that is a large dependency tree (pure Rust git implementation). If you shell out to git: fewer dependencies but subprocess management complexity. Shelling out is probably the right call for v1.

### The Testing Burden Gets Real

Read-only tools need:
- **File system fixtures.** Create temp directories with known file contents, run tools against them, verify output. This is straightforward but verbose.
- **Git fixture repos.** Create a temp git repo with known commits, run git tools, verify output. More complex setup.
- **Agentic loop tests.** The loop (LLM calls tool -> you execute -> LLM gets result -> LLM calls another tool or responds) needs integration tests. You either mock the LLM responses (deterministic but fragile) or call the real API (nondeterministic and costs money).
- **Tool result formatting tests.** The LLM sees tool results as text. If the formatting changes, the LLM behaves differently. These are essentially snapshot tests of your tool output format.

The agentic loop is the hardest to test. Each turn depends on the LLM's decision about which tool to call next. You cannot unit test this meaningfully without mocking.

### Maintenance Burden Rating: Medium

Tools themselves are stable (file systems and git do not change often). But the agentic loop interacts with LLM behavior, which changes with every model update. A model upgrade can change which tools the LLM prefers, how it formats tool calls, and how many turns it takes. You will spend time tuning the system prompt and tool descriptions after every model change.

---

## 3. The "Competitive" Bot

To match Claude Code's experience. This is where the line count goes from hundreds to thousands.

### What Claude Code Actually Is

Claude Code is approximately 163,000-512,000 lines of TypeScript (estimates vary depending on how you count the decompiled npm bundle). The core agent loop file alone (query.ts) is reportedly 46,000 lines handling inference logic, token counting, and chain-of-thought loops. There are 40+ tool modules.

You are not going to match that. Here is what you would need to attempt it.

### Component Estimates

| Component | Lines of Rust | Notes |
|---|---|---|
| Everything from "good enough" | ~1,660 | The foundation |
| Write tools (file create, file edit, file delete) | ~300 | SEARCH/REPLACE block parsing, conflict detection, backup |
| Bash/command execution | ~250 | Subprocess spawning, output capture, timeout management, signal handling |
| Sandboxing | ~400-800 | Restrict what commands can run, file system boundaries, network restrictions. This is hard in Rust without leveraging OS-level sandboxing (seccomp, landlock, macOS sandbox profiles). |
| Sophisticated context management | ~500 | Token counting (tiktoken equivalent in Rust), conversation compaction/summarization, hierarchical memory, message priority scoring |
| Prompt caching integration | ~150 | Cache control headers, breakpoint management, cache invalidation logic |
| Multi-model routing | ~200 | Model selection per message, routing rules, fallback chains |
| Error recovery and retry | ~300 | Mid-stream failure handling, partial response preservation, graceful degradation, exponential backoff with jitter |
| Session persistence | ~200 | Save/resume bot state across restarts, conversation history serialization |
| Permission system | ~300 | Allow/deny lists for tools, user confirmation flows for destructive operations |
| Subagent orchestration | ~400 | Spawn focused child agents (explore, plan, execute), coordinate results |
| LSP integration | ~500+ | Spawn language servers, JSON-RPC communication, hover/definition/references |
| Tree-sitter parsing | ~300 | Multi-language syntax trees, structural code understanding |
| Diff/patch generation | ~200 | Generate unified diffs, apply patches, handle conflicts |
| Project context (CLAUDE.md equivalent) | ~100 | Read and inject project-specific instructions |
| Prompt engineering framework | ~300 | Modular system prompts, conditional components, tool descriptions as behavior instructions |
| **Additional lines beyond "good enough"** | **~4,200-4,600** | |
| **Running total** | **~5,900-6,300** | |

With tests: conservatively double for the testing infrastructure. **Total with tests: ~10,000-12,000 lines.**

That is 6x the size of the current entire bpmux codebase.

### New Dependencies (Partial List)

| Crate | Purpose |
|---|---|
| tiktoken-rs or similar | Token counting |
| tree-sitter + language grammars | Code parsing |
| lsp-types | LSP protocol types |
| nix or similar | Unix process management for sandboxing |
| tower-lsp or custom | LSP client communication |

### The Uncomfortable Truth

Claude Code has a team of engineers at Anthropic maintaining it. They release updates weekly. The CHANGELOG shows continuous iteration on tools, prompts, error handling, and model-specific behavior. They have internal access to model behavior details that no external developer has.

Building a competitive bot is not a one-time cost. It is an ongoing engineering program. If you ship it, you are committing to maintaining it. Every Anthropic model release requires testing and potentially updating the system prompt, tool descriptions, and agentic loop behavior.

### Maintenance Burden Rating: High

This is a product, not a feature. It needs its own roadmap, its own testing infrastructure, its own release cycle. Budget 20-40% of total engineering time on an ongoing basis.

---

## 4. Maintenance Burden: Bot vs Communication Layer

### The Bot (Minimum Version)

| Trigger | Frequency | Effort |
|---|---|---|
| Anthropic model release | ~quarterly | 1-2 days. Update model ID, test system prompt, verify streaming still works. |
| Anthropic API change | ~1-2x/year | 1-3 days. Update client code, test against new behavior. |
| Pricing changes | ~2x/year | 0 code changes, but may want to adjust context window strategy. |
| System prompt tuning | Ongoing | 2-4 hours per round. This never ends. Every model behaves differently. |
| Dependency updates | Monthly (Cargo) | Usually trivial. reqwest is well-maintained. |
| Bug reports about bot behavior | Ongoing | Varies. "The bot said something wrong" is hard to debug because the input is nondeterministic. |

**Annual maintenance estimate for minimum bot: 10-20 days.**

### The Communication Layer (What Exists Today)

| Trigger | Frequency | Effort |
|---|---|---|
| WebSocket protocol changes | You control this | 0 external maintenance |
| MCP spec updates | ~quarterly | Small. rmcp crate handles most of it. |
| TUI framework updates (ratatui) | ~monthly releases | Usually non-breaking. |
| Session format changes | You control this | 0 external maintenance |
| P2P/WebRTC (future) | You control this | Implementation effort, not maintenance |

**Annual maintenance estimate for communication layer: 5-10 days.**

### The Ratio

The bot requires 2-4x more maintenance than the communication layer, and the bot maintenance is reactive (triggered by external changes you do not control) while the communication layer maintenance is proactive (triggered by your own decisions).

---

## 5. The Testing Burden

### How Do You Test an LLM Bot?

The output is nondeterministic. Ask the same question twice, get different answers. This breaks traditional testing approaches.

### What Other Projects Do

**Mock the LLM.** Replace API calls with canned responses in unit/integration tests. This tests your code (message formatting, tool dispatch, error handling) without testing the LLM. The story3_acceptance_test.rs in bpmux already does the real API call approach -- it skips if ANTHROPIC_API_KEY is not set. That is honest but means CI cannot run the test without paying for API calls.

**Snapshot testing.** Record a real API interaction (request + response), save it as a fixture, replay it in tests. Tests are deterministic but brittle -- they break when you change the system prompt or message format. LangChain and LangSmith use this approach with evaluation datasets.

**Semantic assertions.** Instead of asserting exact output, assert properties: "response mentions the number 4", "response is under 200 tokens", "response does not contain code blocks." Looser but more resilient to model changes.

**LLM-as-judge.** Use a separate LLM call to evaluate whether the response is good. Expensive, slow, and itself nondeterministic. Used in production eval pipelines, not in CI.

**Integration tests with real API calls.** The honest approach. Run the test, pay the cost. Aider runs benchmarks against real models as part of their release process. These are not unit tests -- they are acceptance tests that run periodically, not on every commit.

### The Practical Testing Stack for bpmux

| Layer | What to Test | How | Cost |
|---|---|---|---|
| SSE parser | Correctly parses Anthropic stream events | Unit tests with fixture data | Free |
| Message formatting | System prompt + history -> correct API request | Unit tests with snapshots | Free |
| Conversation window | Sliding window, token budgeting | Unit tests | Free |
| Tool dispatch | Correct tool called with correct args | Unit tests with mock tool results | Free |
| Agentic loop | Multi-turn tool use converges | Integration test with mocked LLM | Free |
| Bot behavior | Actually useful responses | Acceptance test with real API | ~$0.02-0.10 per test run |
| End-to-end | Human types, bot responds in TUI | PTY test (like story3) with real API | ~$0.02-0.10 per test run |

The expensive tests (real API) should not run in CI on every push. They run manually or on a schedule. Budget $5-10/month for periodic acceptance testing.

### The Real Testing Problem

The hard part is not writing tests. It is knowing when the bot has regressed. If you upgrade from Sonnet 4 to Sonnet 4.6, the bot might subtly change behavior: it might be more verbose, choose different tools, format responses differently. None of your snapshot tests catch this because you need new snapshots. None of your unit tests catch this because they test your code, not the model. Only human judgment catches behavioral regression.

This is why Claude Code has a team. This is why Aider runs benchmarks on SWE-bench. This is the maintenance that never ends.

---

## 6. Scope Creep Risk

### The Research Warning

The prior research recommended: "Start with conversation, add read-only tools, defer write tools." That is three tiers of scope already planned before shipping.

### The Precedent

Every AI coding tool that started minimal grew complex:

- **Aider** started as a simple chat-to-edit tool. Now has: multiple edit formats (diff, whole-file, architect mode), repo map generation via tree-sitter, multi-file editing, git integration, linting, voice input, browser support, and 15+ model provider integrations.

- **OpenCode** started as a terminal coding agent. Now has: LSP integration, git snapshots at each step for rollback, automatic summarization, provider-aware prompts, and a TUI with multiple panels. 95K+ GitHub stars, significant engineering investment.

- **Cursor** started as a VS Code fork with AI chat. Now ships: multi-file editing, codebase indexing, custom models, agent mode, background agents, and memory systems. Raised $900M+ in funding.

None of these stayed minimal. The market rewards capability. Users ask for the next thing and the next thing.

### The Specific Requests That Will Come

If bpmux ships a minimum chat bot:

1. "The bot should be able to read my code" (read-only tools -- the first escalation)
2. "The bot should know what file I am looking at" (editor integration)
3. "The bot should be able to run my tests" (command execution -- crossing the write boundary)
4. "The bot should fix the bug it found" (file editing)
5. "The bot should remember what we discussed yesterday" (persistent memory)
6. "The bot should work with my OpenAI key, not just Anthropic" (multi-provider)
7. "The bot is too slow, can it use a smaller model for simple questions?" (model routing)
8. "The bot should work offline with a local model" (local inference)

Each request is reasonable. Each makes the bot better. Together, they are Claude Code.

### How to Maintain Discipline

**Draw the line in architecture, not in willpower.** The bot connects via WebSocket. It is a separate process. It does not get special access. If you want a better bot, swap it -- do not grow the built-in one.

**Make the line visible.** Document what the built-in bot does and does not do. Put it in the README, the help text, the first-run message. "The built-in bot is a chat participant. For coding assistance, connect Claude Code or your preferred AI tool."

**Measure against the alternative.** Every time someone asks for a bot feature, ask: "Is this better solved by improving the communication protocol so external tools work better, or by growing the internal bot?" Usually it is the former.

**Accept that some users will leave.** Not every user wants to set up Claude Code. Some will try bpmux, find the built-in bot insufficient, and leave. That is acceptable if the users who stay are the ones who value the communication layer.

---

## 7. The Opportunity Cost

### What the Bot Costs in Time

The minimum bot (Section 1) is roughly 2-3 weeks of focused development, including tests. The "good enough" bot (Section 2) is 4-6 weeks. The "competitive" bot (Section 3) is 3-6 months.

### What You Are Not Building During That Time

| Feature | Estimated Effort | What It Gives bpmux |
|---|---|---|
| P2P/WebRTC mesh networking | 4-8 weeks | The core differentiator. bpmux without P2P is just another local chat tool. |
| Web UI client | 3-5 weeks | Browser access to bpmux sessions. Massively expands reach. |
| Session management (join/leave/resume) | 2-3 weeks | Multi-session support, reconnection, session discovery. |
| Security (authentication, encryption) | 3-5 weeks | Required for any networked deployment. Currently zero auth. |
| search_messages tool | 1-2 weeks | Full-text search across session history. Useful for all participants, human and AI. |
| Message threading/topics | 2-3 weeks | Organize conversation by topic. Critical at scale. |
| File/code sharing in chat | 1-2 weeks | Share code snippets, diffs, files within the session. |
| Participant presence/status | 1 week | Who is online, who is typing, connection status. |
| Session recording/replay | 2-3 weeks | Record sessions for review, onboarding, documentation. |

The minimum bot takes the same time as session management, participant presence, and search_messages combined. The "good enough" bot takes the same time as P2P networking.

### The Tradeoff in Plain Terms

Every week on the bot is a week not spent on what makes bpmux different from every other AI chat wrapper. There are dozens of AI chat bots. There is no P2P group chat for AI pair programming sessions.

---

## 8. The Honest Comparison

### Path A: Build the Bot (3-Month Horizon)

**What you ship:** bpmux with a built-in chat bot that reads the conversation, calls Claude, posts responses. Maybe read-only tools if time permits.

**What users get:** A single-machine chat tool where you can talk to Claude with a TUI instead of a browser. No P2P. No multi-machine sessions. No web access.

**The competitive landscape:** This competes with Claude Code (better at coding), Aider (more features), OpenCode (free, 75+ models), ChatGPT (more accessible), and every other AI chat interface. You are competing on the AI experience, which means competing with Anthropic, OpenAI, and well-funded startups. You lose.

**What is unique:** Nothing. A Rust TUI for Claude chat is a weekend project on Hacker News, not a product.

### Path B: Focus on Communication (3-Month Horizon)

**What you ship:** bpmux with P2P networking (or at minimum, multi-machine sessions over WebSocket relay), session persistence, reconnection, basic auth, and a protocol that external AI tools connect to.

**What users get:** A group chat where multiple developers and multiple AI tools collaborate in the same session. Claude Code connects via MCP. A custom bot connects via WebSocket. A web browser connects via the web UI. The session persists across disconnections. Multiple machines participate.

**The competitive landscape:** There is no product that does this. Claude Code is single-user. Aider is single-user. OpenCode is single-user. Slack/Discord have AI bots but are not designed for pair programming. The communication infrastructure is the moat.

**What is unique:** Everything. Nobody else is building the room.

### The Hybrid Path

Build the minimum bot (2-3 weeks) and then stop. Use the remaining 9-10 weeks on communication infrastructure.

The minimum bot serves as:
- A reference implementation of the WebSocket bot protocol
- An onboarding device (empty room problem)
- A forcing function for the streaming message UX
- Proof that the protocol works

It does NOT serve as:
- A competitive AI coding assistant
- The primary way users interact with AI in bpmux
- A feature that grows over time

The discipline required: when users ask for the next thing (read-only tools, write tools, model routing), the answer is "connect Claude Code" or "build a bot against the WebSocket API." The built-in bot stays at minimum.

### The Risk of the Hybrid Path

The risk is that the minimum bot is underwhelming and users judge bpmux by its weakest feature rather than its strongest. Prior research assessed this as "medium confidence" and noted that "without at least basic tool use (reading the current file), the bot is too dumb to demonstrate pair programming value."

That risk is real. But it is smaller than the risk of spending 3 months building a bot and shipping a product that has no differentiator.

---

## Summary Table

| Dimension | Minimum Bot | Good Enough Bot | Competitive Bot |
|---|---|---|---|
| Lines of Rust (with tests) | ~1,020 | ~2,460 | ~10,000-12,000 |
| New production dependencies | 3 | 6-8 | 12-15 |
| Build time | 2-3 weeks | 4-6 weeks | 3-6 months |
| Annual maintenance | 10-20 days | 20-40 days | 60-100 days |
| Relative to current codebase | 1x (doubles it) | 2.3x | 10-12x |
| Testing approach | Mostly unit, 1-2 acceptance | Mixed unit/integration, mocked agentic loop | Full test infrastructure, eval pipeline |
| API cost for testing | $5-10/month | $10-20/month | $20-50/month |
| Scope creep risk | Medium | High | Certain |
| Opportunity cost | Session management + search | P2P networking | The whole product |

### The Honest Answer

The minimum bot is affordable. 2-3 weeks, ~1,000 lines, manageable maintenance. It solves the empty room problem and demonstrates the protocol.

The "good enough" bot is where the trap is. It sounds reasonable ("just add file reading") but it doubles the code, introduces the agentic loop (the hardest thing to test and maintain), and starts the scope creep flywheel.

The "competitive" bot is a different product. Do not build it.

The communication layer is what makes bpmux worth existing. Every hour on the bot is defensible only if the communication layer is already solid enough to demonstrate the product's value. Right now it is not. There is no P2P. There is no multi-machine support. There is no web UI. There is no auth.

Build the minimum bot. Then build the room.
