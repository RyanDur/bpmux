# Editor Notes: AI Participant Design Proposal

Date: 2026-04-02
Reviewed by: Editorial pass (strict)

---

## Overall Assessment

The proposal is well-structured and makes a clear architectural recommendation. The core thesis -- bpmux should own the conversation loop via direct API calls rather than relying on MCP polling or the Agent SDK -- is well-supported by the research. However, there are factual inaccuracies, unsourced claims, missing research, and an effort estimate that does not hold up under scrutiny.

---

## 1. Claims vs. Research: What Checks Out

**Supported claims:**

- "Every AI chat tool uses the same loop" -- the inspirations research (Section 5 of the inspirations doc) and the competitor analysis confirm this. The REPL pattern is universal.
- "The Claude Agent SDK is wrong for bpmux" -- the tool-execution research (Agent 7) explicitly recommends against the Agent SDK sidecar for v1. The language mismatch and Claude-only lock-in are documented.
- "OpenAI Chat Completions is the universal interface" -- the rust-ecosystem research and competitor analysis confirm wide adoption. The `llm` crate covering 10+ providers is documented.
- "A2A is task-oriented, not conversation-oriented" -- the A2A protocol research explicitly concludes this: "Not relevant for v1."
- "Conversation history replay is how AI remembers" -- confirmed by the inspirations research and the context-management research.
- "Provider abstraction is a solved pattern" -- the rust-ecosystem research documents the `llm` crate, `rig`, and others. OpenCode's two-layer pattern is documented.

---

## 2. Claims vs. Research: What Does NOT Check Out

### 2a. "The Agent SDK is task-oriented" (Finding 2, bullet 3)

The proposal says: "The SDK runs a task to completion. bpmux needs an ongoing conversation participant, not a one-shot task runner."

This is misleading. The Agent SDK supports multi-turn conversations and session management. Claude Code itself -- which is built on the Agent SDK -- runs ongoing interactive sessions, not one-shot tasks. The tool-execution research (Agent 7) does recommend against the SDK, but for different reasons: language mismatch (Python/TypeScript only) and Claude-only lock-in. Framing it as "task-oriented" mischaracterizes the SDK's capabilities and weakens the argument. The real reasons are strong enough on their own.

### 2b. "Approximately 300-500 lines of Rust for a working client" (Day 1 Anthropic)

This claim is unsourced. No research file provides this estimate. The rust-ecosystem research mentions "~500-700 lines" for a direct MCP protocol implementation as an alternative to rmcp, but that is a different component entirely. The context-management research describes token counting, compaction, caching, and cost tracking -- none of which are trivially small. The tool-execution research shows a tool executor skeleton alone at ~100 lines, and that is a sketch without error handling, tests, or real-world edge cases.

A working Anthropic streaming client in Rust requires: HTTP request construction, SSE stream parsing, JSON deserialization of the Messages API response format (which includes content blocks, tool_use, token usage, and stop reasons), error handling for rate limits and API errors, and retry logic. 300-500 lines is plausible for a bare-minimum non-streaming client that sends messages and gets text back. It is not plausible for a production-quality streaming client with proper error handling, which is what the proposal actually describes.

### 2c. "Another 200-300 lines" for OpenAI-compat (Day 2)

Same problem. Unsourced. The research does not estimate line counts for provider implementations. The OpenAI Chat Completions API has a different response format (choices array, delta objects in streaming, different error shapes). 200-300 lines assumes nearly complete code reuse with the Anthropic client, which is unlikely given different SSE event formats and response structures.

### 2d. A2A description omits key nuance

The proposal says A2A has "no group/broadcast primitive." The A2A research file does not make this specific claim. The research says A2A is "task delegation (client-server)" vs bpmux's "shared conversation (P2P)" and that A2A is for "agents delegating structured tasks to other agents across organizational boundaries." The conclusion is correct but the specific claim about missing group/broadcast is not in the sources.

### 2e. "MUCA research shows multi-party AI chat is possible but understudied" (Appendix, Source 5)

The MUCA reference appears in the appendix but is not cited or discussed in any research file I can find. This appears to be an unsourced claim presented as if it came from the research.

---

## 3. Unsourced Claims Presented as Facts

- "API calls take 1-5 seconds for the first token" (Risk 2: Latency) -- no research file provides this latency range. The context-management research discusses caching reducing latency by up to 85%, which implies the uncached baseline is significant, but no specific numbers are given.
- "Claude Code has thousands of lines dedicated to [context window management]" (Risk 4) -- plausible but unsourced. None of the research agents examined Claude Code's source to verify this.
- "reqwest + serde + eventsource-stream stack handles it" (Finding 6) -- the rust-ecosystem research recommends this stack conceptually, but `eventsource-stream` specifically is not evaluated in the crate assessment. It appears nowhere in the rust-ecosystem research.
- "50 messages provides sufficient context" (Hypothesis) -- the context-management research recommends "5-15 recent turns" based on the JetBrains research, and "50-100K tokens" as a budget ceiling. The number 50 is not derived from any source. It appears to be an arbitrary default.

---

## 4. Reasoning Gaps

### 4a. The proposal ignores the inspirations research's richer architecture

The inspirations research (2026-04-02-ai-participant-inspirations.md) recommends a significantly more sophisticated architecture than the proposal adopts:

- **Actor-per-participant with tokio broadcast channel** -- the inspirations research explicitly recommends this. The proposal mentions broadcast channels only in the diagram but does not discuss the actor model or how the AI participant integrates into the existing tokio task architecture.
- **Batched perception, not per-message** -- the inspirations research recommends the AI accumulate messages and evaluate in batches at natural pause points. The proposal's trigger policy (Option D) mentions silence detection but does not discuss batching. The "respond when addressed" default skips this entirely.
- **Behavior tree decision model** -- the inspirations research recommends a behavior tree with turn-taking rules from linguistics research. The proposal reduces this to a simple trigger policy. This is arguably appropriate for v1, but the proposal does not acknowledge the simplification or explain why the richer model is deferred.
- **Contribution quality gate** -- the Koala research finding that 72% of participants preferred reactive mode, and the Inner Thoughts motivation scoring, both suggest the AI should self-filter before posting. The proposal mentions the Koala finding indirectly in "Risk 6" but does not incorporate it into the design.

The proposal cherry-picks the simplest elements from the inspirations research without acknowledging what it leaves out or why.

### 4b. Server-side compaction is not mentioned

The context-management research strongly recommends Anthropic's server-side compaction API as "the recommended approach for new implementations." The proposal's context management strategy is "sliding window, simple truncation, defer compaction." This ignores the research's primary recommendation. If bpmux is calling the Anthropic API directly, using the server-side compaction API (a single field in the request) would be dramatically simpler than building client-side truncation logic.

### 4c. Prompt caching is not mentioned

The context-management research dedicates an entire section to prompt caching and calculates that it yields 90% savings on static prefixes. This is directly relevant to the cost risk (Risk 1) and the system prompt design. A stable system prompt cached across API calls would significantly reduce per-request cost. The proposal discusses cost mitigation (use Sonnet, display estimates, budget caps) but misses the single largest cost reduction technique documented in its own research.

### 4d. The "two paths" risk is understated

Risk 3 acknowledges the confusion of having MCP-client and bpmux-managed AI paths. But the proposal does not address a deeper question: if bpmux manages the AI participant directly, what happens to the MCP server's `read_messages` tool? Currently an MCP client reads all messages. With a bpmux-managed AI also consuming messages and responding, an external MCP client (like Claude Code) would see the bpmux-managed AI's responses in `read_messages` -- potentially creating a situation where two AIs are both trying to participate, one managed by bpmux and one external, without coordination. The proposal says "they serve different use cases and can coexist" but does not explain how they coexist without conflicting.

---

## 5. Cost Estimates: Incomplete

The proposal says cost is a real risk but does not attempt an estimate, saying "we need to measure this in practice." The research provides enough data to produce a rough estimate:

From the context-management research:
- Opus input: $5/M tokens uncached, $0.50/M cached
- Opus output: $25/M tokens
- Sonnet pricing is lower (not specified in the research, but the proposal recommends Sonnet as default)

A typical exchange: 50 messages at ~100 tokens each = 5,000 input tokens. Plus system prompt (~500 tokens). AI response ~500 tokens.

Per-response cost (Opus, uncached): ~$0.03 input + ~$0.01 output = ~$0.04
Per-response cost (Opus, cached system prompt): lower, but message history is not cacheable across calls since it changes.

A 30-minute pair programming session with 50 AI responses: ~$2.00 with Opus.

This is a back-of-envelope calculation the proposal should have included, even approximately. "We need to measure" is not acceptable when the research provides the pricing data.

---

## 6. Effort Estimate: 500-800 Lines Is Not Credible

The proposal claims the minimal first step is "~500-800 lines of new Rust code across 3-4 files." Let me break down what the "minimal first step" actually requires:

1. **`provider.rs`** -- `AiProvider` trait, `CompletionRequest`, `CompletionResponse`, `StreamChunk`, `ChatMessage`, `Role`, `TokenUsage`, `FinishReason` types. With derives, impls, and error types: ~80-120 lines.

2. **`anthropic.rs`** -- Anthropic Messages API client:
   - Request/response structs matching the Anthropic API format (different from the provider trait's types): ~60-80 lines
   - `send()` implementation: HTTP POST, deserialize response, map to provider types: ~40-60 lines
   - `stream()` implementation: HTTP POST with Accept: text/event-stream, SSE parsing, map delta events to StreamChunk: ~80-120 lines
   - Error handling (rate limits, API errors, network errors): ~40-60 lines
   - Authentication (API key from env var): ~20-30 lines
   - Total: ~240-350 lines minimum

3. **`participant.rs`** -- orchestrator:
   - Watch the message log for new messages (subscribe to broadcast channel): ~30-40 lines
   - Trigger logic (detect @ai mentions): ~30-40 lines
   - Context assembly (collect recent messages, format as ChatMessage array, build system prompt): ~60-80 lines
   - Call provider, stream response into log: ~50-70 lines
   - Error handling, timeout, cancellation: ~40-60 lines
   - Total: ~210-290 lines minimum

4. **`context.rs`** (the proposal lists this file but then says it is deferred. If deferred, it is not in the 500-800 estimate. If not deferred, add ~100-150 lines for basic sliding window).

5. **Integration into `lib.rs`**: spawn AI participant as a tokio task, pass broadcast receiver, pass session dir, handle configuration: ~30-50 lines.

6. **Tests**: The codebase has tests alongside every module. The proposal's minimal step includes proving the end-to-end flow. At minimum: unit tests for trigger detection, context assembly, response parsing. Integration test for the full loop. Conservatively: ~200-400 lines of tests.

**Total without tests:** ~510-810 lines. This matches the estimate -- but only if you exclude tests entirely and accept zero error handling beyond basic Result propagation.

**Total with tests (matching the codebase's existing test discipline):** ~710-1210 lines.

**Total with realistic error handling, retry logic, and configuration:** ~900-1400 lines.

The 500-800 estimate is achievable only if you define "working" very loosely -- no tests, minimal error handling, no configuration, no retry logic. Given the codebase's existing standards (tests in every module, `thiserror` error types, no `unwrap()` in production), the realistic estimate is closer to 1000-1400 lines for a production-quality minimal slice.

---

## 7. Missing from the Proposal That the Research Covered

### 7a. The Koala research finding on user control

The inspirations research documents the Koala study finding that 72% preferred reactive mode and that users wanted: a mode toggle, a contribution threshold slider, and the ability to relegate responses to threads. The proposal's trigger policy captures the mode toggle but drops the threshold slider and the thread-relegation concept. The threshold idea (AI scores its own response and suppresses low-scoring ones) is described as "directly implementable" in the inspirations doc but absent from the proposal.

### 7b. Turn-taking from linguistics

The inspirations research has a detailed section on Sacks, Schegloff, and Jefferson's turn-taking model and how it applies to AI conversation. The proposal's trigger policy is a crude approximation. The research's specific insight -- that the AI should be the participant who "speaks least, not most" -- is not reflected in the proposal's design.

### 7c. The "navigator, not driver" role definition

The inspirations research recommends the AI default to a pair-programming navigator role with specific guidelines about when to speak and when to stay quiet. The proposal's system prompt section includes a generic set of guidelines but does not incorporate the navigator role or the specific heuristics from the research.

### 7d. Memory retrieval from the Stanford Generative Agents research

The inspirations research recommends a memory retrieval model weighted by recency and relevance, not just a flat sliding window. The proposal defaults to a flat window of the last N messages. This is reasonable for v1 but the proposal does not acknowledge the research finding or explain why it was deferred.

### 7e. The `tokio::sync::broadcast` channel recommendation

The inspirations research and the async-patterns research both recommend `tokio::sync::broadcast` for the observer pattern -- the AI subscribes to the same message stream as the TUI. The proposal's architecture diagram shows this implicitly but does not name the mechanism. The existing codebase already uses `broadcast::channel` for wake signals. The proposal should have connected to this existing infrastructure explicitly.

### 7f. Anthropic's count_tokens API

The context-management research documents a free `count_tokens` endpoint. The proposal mentions "track token usage from API responses" but does not mention the count_tokens API for proactive budget management before sending a request.

---

## 8. Does the Proposal Address the Actual Problem?

The problem statement is: "MCP is request/response. The AI can only see messages when it actively calls `read_messages`. It cannot receive messages in real time."

**Partially.** The proposal solves the "AI cannot respond spontaneously" problem by having bpmux call the AI when triggered. But it does not make the AI a real-time participant in the way the problem statement implies. The AI still does not "receive messages as they arrive." It receives a batch of messages when bpmux decides to invoke it. The proposal acknowledges this ("The AI 'sees every message' not because it is listening, but because the application replays the entire conversation") but frames it as a feature rather than a limitation.

This is an honest reframing, not a dodge. The proposal correctly identifies that no AI tool has persistent real-time message reception -- it is architecturally impossible with current LLM APIs. The proposal should be more explicit that it is redefining the problem: from "the AI should receive messages in real time" to "the AI should respond at conversationally appropriate times, and bpmux (not the external tool) decides when those times are."

---

## 9. Minor Issues

- The `model = "claude-sonnet-4-20250514"` in the configuration example is a specific model version. The research recommends Sonnet as the cost-effective default, but the proposal does not explain why this specific version or how model version updates would be handled.
- The `silence_ms = 3000` default is presented without justification. The inspirations research cites 700ms as noticeably long in human conversation and "N seconds" for text chat, but does not recommend 3 seconds specifically. The Inner Thoughts research uses 10 seconds.
- The proposal says "No official Rust SDK but community options exist" (Appendix, Source 6). The rust-ecosystem research names several: `misanthropy`, `anthropic-sdk-rust`, `claude-sdk`. The proposal should cite these consistently.

---

## Summary: What Needs Fixing Before This Becomes an Implementation Plan

1. **Remove or correct the "task-oriented" characterization of the Agent SDK.** The real reasons (language mismatch, provider lock-in) are sufficient.
2. **Source the line-count estimates or remove them.** Replace with honest ranges that account for tests and error handling. ~1000-1400 lines is more credible.
3. **Add a rough cost estimate.** The research provides the pricing data. Even a back-of-envelope calculation is better than "we need to measure."
4. **Mention server-side compaction and prompt caching.** These are the research's top recommendations for context management and cost reduction. Omitting them from the design is a significant gap.
5. **Address AI participant coexistence with MCP clients.** Two AIs in the same session need coordination, or at least the proposal needs to acknowledge the risk concretely.
6. **Incorporate (or explicitly defer with rationale) the richer architecture from the inspirations research.** The trigger policy is the most important design decision in this proposal and it gets the least design rigor.
7. **Connect to the existing codebase.** The proposal describes `src/ai/` as a new bounded context but does not reference the existing `broadcast::channel` in `lib.rs` or explain how the AI participant task fits into the existing architecture (main thread TUI + spawned MCP server thread).
8. **Source the MUCA claim or remove it.**
