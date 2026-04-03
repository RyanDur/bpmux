# Context Window Management Research

Research for bpmux: if the system calls AI APIs directly, what does it take to
manage the context window responsibly?

---

## 1. Token Counting in Rust

### Anthropic's Official API (server-side, authoritative)

Anthropic provides a free `POST /v1/messages/count_tokens` endpoint that accepts
the exact same payload as the Messages API (system, tools, messages) and returns
`{ "input_tokens": N }`. This is the ground-truth count -- it accounts for
internal formatting that no client library can replicate exactly. Rate limits
are separate from message creation (100-8000 RPM depending on tier).

Important caveat from the docs: the count is an **estimate** and may include
tokens added by Anthropic for system optimizations. You are not billed for
system-added tokens.

### Rust Crates for Client-Side Estimation

**claude-tokenizer** (`crates.io/crates/claude-tokenizer`, v0.3.0)
- Provides `count_tokens(text)` for offline estimation
- Uses a tokenization vocabulary file; accuracy is approximate
- MIT licensed, small dependency footprint
- Unknown whether the embedded vocab file is officially sanctioned by Anthropic

**tiktoken-rs** (`crates.io/crates/tiktoken-rs`)
- Pure Rust port of OpenAI's tiktoken
- Supports o200k_base, cl100k_base, p50k_base, r50k_base encodings
- Well-maintained (zurawiki/tiktoken-rs), good docs
- Only accurate for OpenAI models; Claude uses a different tokenizer

### Practical Recommendation for bpmux

Use the official count_tokens API for anything that matters (cost estimation,
deciding when to compact). Use a local Rust tokenizer only for rough heuristics
where you need sub-millisecond latency and can tolerate ~10-20% error (e.g., "is
this message probably over 4K tokens?"). For Claude specifically, the
claude-tokenizer crate is the closest match, but treat it as advisory.

---

## 2. Compaction Strategies

### How Claude Code Does It

Claude Code uses a multi-layer approach:

1. **Tool result clearing** (lightest touch) -- once a tool call result is deep
   in conversation history, the raw output is dropped. This is the safest form
   of compaction because the model has already acted on the information.

2. **Conversation summarization** -- when context reaches ~95% capacity (or the
   configurable threshold), the entire message history is passed to Claude with a
   summary prompt. Claude generates a structured summary wrapped in
   `<summary>` tags. The SDK extracts the summary and replaces the full history.
   This preserves architectural decisions, unresolved bugs, and implementation
   details while discarding redundant tool outputs.

3. **Surgical compaction** -- Claude Code compresses trial-and-error sessions
   (failed attempts, retries) while preserving early context that established
   the task. Reported to reclaim 80-90% of token cost while keeping the
   essential narrative.

Claude Code recently reduced its autocompact buffer from 45K to 33K tokens,
meaning it triggers compaction earlier to leave more room for output.

### Anthropic's Server-Side Compaction API (Beta)

As of January 2026, Anthropic offers a server-side compaction feature via the
Messages API:

- Enable with `context_management.edits: [{ "type": "compact_20260112" }]`
- Requires beta header `compact-2026-01-12`
- Supported on Claude Opus 4.6 and Claude Sonnet 4.6
- Configurable trigger threshold (token count)
- When triggered, the API generates a `compaction` block in the response
- On subsequent requests, the API automatically drops messages prior to the
  compaction block

This is the **recommended** approach for new implementations. It offloads the
complexity of summary generation, threshold management, and message pruning
entirely to Anthropic's servers.

### How Aider Manages Context

Aider takes a fundamentally different approach -- it does not maintain
conversation history the way a chat app does. Instead:

- **Repository map**: Uses tree-sitter to parse source files, extracts
  definitions and references, builds a dependency graph, and ranks files by
  relevance using PageRank-style graph analysis
- **Dynamic token budgeting**: The repo map is rebuilt before every LLM call,
  with a default budget of 1K tokens (8x when no files are in chat)
- **Binary search fitting**: Aider uses binary search to find the largest set of
  relevant code entities that fits within the active token budget

Aider does not summarize old messages -- it treats each interaction as largely
stateless, relying on the repo map + explicitly added files for context.

### JetBrains Research: The Complexity Trap (NeurIPS 2025)

Key finding: **simple observation masking matches LLM summarization** for agent
context management, at lower cost.

- Observation masking: truncate or drop tool outputs beyond a window of recent
  turns. No LLM call needed.
- LLM summarization: call the model to compress history. More expensive and
  causes "trajectory elongation" -- agents run ~15% more steps because the
  summary loses action-relevant details.
- A hybrid approach (mask + selective summarization) reduced costs by 7-11%
  compared to either strategy alone.
- Tested on OpenHands and SWE-agent; observation masking required per-scaffold
  tuning of the window hyperparameter.

---

## 3. The Sliding Window + Summary Pattern

### The Standard Pattern

1. Keep the last N messages (or last K tokens) verbatim
2. Everything older gets summarized into a single "context summary" block
3. The summary is placed at the start of the conversation as a system message or
   first user message
4. As new messages arrive, the window slides and the summary is regenerated
   (or incrementally updated)

### Generating the Summary

Yes, you call the AI itself. The typical approach:

- Construct a prompt like: "Summarize this conversation, preserving: key
  decisions made, unresolved questions, current task state, any file
  paths/names/identifiers mentioned."
- Use a cheaper/faster model if available (e.g., Haiku for summarization, Opus
  for the actual work)
- The summary output becomes the new "history prefix" for subsequent turns

### Hierarchical Summarization (for very long sessions)

From the mem0.ai guide and OpenAI's patterns:

- Keep last 10 messages verbatim
- Messages 11-20: condensed summary
- Messages 20+: high-level summary of summaries
- This creates a "pyramid" where recent context is high-resolution and older
  context is progressively compressed

### Practical Concerns

- **Information loss is inevitable.** Every compaction loses something. The
  question is whether it loses things that matter for the current task.
- **Summary drift.** Repeated summarization of summaries compounds errors. After
  3-4 rounds, the summary may not faithfully represent the original conversation.
- **Cost of summarization.** Each compaction requires an LLM call. If using the
  same model, this can be 10-50K tokens of input just for the summary step.
- **Anthropic's server-side compaction** handles all of this for you. For bpmux,
  this is almost certainly the right starting point.

---

## 4. Cost Tracking

### The Usage Object in API Responses

Every Claude Messages API response includes a `usage` object:

```json
{
  "usage": {
    "input_tokens": 500,
    "output_tokens": 1200,
    "cache_creation_input_tokens": 50000,
    "cache_read_input_tokens": 100000
  }
}
```

**Relationship:** `total_input = cache_read + cache_creation + input_tokens`

Note: `input_tokens` represents only tokens *after* the last cache breakpoint,
not all input tokens sent. When caching is active, this number is much smaller
than total input.

### Computing Cost Per Request

```
cost = (input_tokens / 1M * input_price)
     + (cache_creation_input_tokens / 1M * cache_write_price)
     + (cache_read_input_tokens / 1M * cache_read_price)
     + (output_tokens / 1M * output_price)
```

Current prices per million tokens (Claude Opus 4.6):

| Category | Price/M tokens |
|----------|---------------|
| Input (uncached) | $5.00 |
| Cache write (5m) | $6.25 |
| Cache write (1h) | $10.00 |
| Cache read | $0.50 |
| Output | $25.00 |

### Tracking Cumulative Cost in a Session

The straightforward approach: accumulate the usage fields across all API calls in
a session struct:

```rust
struct SessionCost {
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cache_creation_tokens: u64,
    total_cache_read_tokens: u64,
    total_cost_usd: f64,  // computed from the above
}
```

Update after every API response. Anthropic also provides organization-level
reporting via `/v1/organizations/usage_report/messages` with time-bucket
aggregation (1m/1h/1d), but for per-session tracking, accumulating from
responses is the standard approach.

---

## 5. Prompt Caching

### How It Works

Anthropic caches the internal model state for a prefix of your prompt. On
subsequent requests with an identical prefix, the model skips recomputation of
those tokens, yielding up to 85% latency reduction and 90% cost reduction on
cached portions.

### What to Cache (in order of stability)

1. **Tool definitions** (`tools` array) -- most stable, changes rarely
2. **System prompt** -- changes infrequently within a session
3. **Conversation history** -- grows each turn but prefix remains stable
4. **Documents/images in context** -- if doing RAG-style inclusion

### Cache Control Mechanisms

**Automatic caching** (recommended for multi-turn):
```json
{ "cache_control": { "type": "ephemeral" } }
```
Place at the top level of the request. The system automatically applies cache
breakpoints to the last cacheable block and moves them forward as conversation
grows.

**Explicit breakpoints** (fine-grained control):
Place `cache_control` on individual content blocks. Up to 4 breakpoints allowed
per request, for caching sections that change at different rates.

### Minimum Token Requirements

| Model | Minimum cached prefix |
|-------|----------------------|
| Opus 4.6, Opus 4.5, Haiku 4.5 | 4,096 tokens |
| Sonnet 4.6, Haiku 3.5 | 2,048 tokens |
| Sonnet 4.5, Sonnet 4, Opus 4.1, Opus 4 | 1,024 tokens |

Shorter prompts process normally without caching (no error).

### TTL Options

- **5-minute** (default): `{ "type": "ephemeral" }` -- 1.25x base input price
  for writes
- **1-hour**: `{ "type": "ephemeral", "ttl": "1h" }` -- 2x base input price for
  writes
- Both: 0.1x base input price for reads (90% savings)

### Cache Invalidation

Cache requires 100% exact match of all content up to and including the marked
block. Any change to tool definitions, system prompt text, or message content
before the breakpoint invalidates the cache.

### Relevance to bpmux

For a P2P group chat calling the API, the system prompt and tool definitions are
prime candidates for caching. In a typical session:
- First call: cache write (~1.25x cost for those tokens)
- Subsequent calls: cache read (~0.1x cost) -- a 92% savings on the static
  prefix

With a system prompt + tools totaling ~2-5K tokens, this saves meaningfully over
a session of dozens of calls.

---

## 6. Practical Context Limits

### The Gap Between Advertised and Effective

Research consistently shows models degrade well before hitting their advertised
context window:

**"The Complexity Trap" (NeurIPS 2025):** Every model tested exhibits
degradation as input length increases. Some top-line models showed severe
degradation with as little as 1,000 tokens of context -- falling far short of
their maximum context window by as much as 99%.

**Chroma "Context Rot" study:** Tested 18 frontier models. Every single one
exhibited measurable performance degradation as input context grew. This happens
even when the context window is nowhere near full.

**"Lost in the Middle" phenomenon:** Models attend strongly to the beginning and
end of context but poorly to the middle. In multi-document QA with 20 documents,
accuracy dropped 30%+ when relevant information was in positions 5-15 vs.
position 1 or 20. This follows a U-shaped attention curve.

**RULER benchmark (NVIDIA):** Despite near-perfect scores on simple
needle-in-a-haystack retrieval, most models show large degradation on more
complex tasks (multi-hop tracing, aggregation, QA) as sequence length increases.

**Context Discipline study (Jan 2026):** Analyzed performance correlation under
varying context lengths. Found that attention budget gets spread thinner as
context grows -- information highly attended at 1K tokens becomes functionally
ignored at 100K tokens.

### Practical Numbers

- Claude Opus 4.6 / Sonnet 4.6: 1M token context window (GA as of March 2026)
- Effective reliable performance: significantly less. No published number from
  Anthropic, but external benchmarks suggest degradation begins around 100-200K
  tokens for complex tasks
- Claude Code's autocompact triggers at ~95% capacity but the 33K buffer
  suggests they want substantial headroom
- For coding agents, the JetBrains research found optimal context windows of
  5-15 recent turns (not token-based, turn-based)

### Root Cause

Transformer attention scales quadratically with sequence length. As context
grows:
- Computational cost increases (though this is Anthropic's problem, not yours)
- The model's finite attention budget gets spread across more tokens
- Training data disproportionately features shorter sequences
- Information retrieval accuracy drops, especially for middle-positioned content

### What This Means for bpmux

1. **Do not rely on large context as a substitute for good context management.**
   Even with 1M tokens available, performance degrades long before that.

2. **Target 50-100K tokens of active context** as a practical ceiling for
   reliable performance on reasoning-heavy tasks.

3. **Use compaction proactively**, not just when you hit the limit. Shorter,
   focused context produces better results than longer, everything-in context.

4. **Place the most important information at the beginning and end** of the
   context. The middle is the weakest position for retrieval.

5. **Anthropic's server-side compaction** is the simplest path. It handles
   threshold detection, summary generation, and message pruning. bpmux can use
   it by adding a single field to API requests.

---

## Summary of Recommendations for bpmux

| Concern | Recommendation |
|---------|---------------|
| Token counting | Use Anthropic's count_tokens API for accuracy; claude-tokenizer crate for fast local estimates |
| Compaction | Start with Anthropic's server-side compaction API (beta). Fall back to client-side sliding window + summary if you need more control |
| Summary generation | Use the AI itself (cheaper model if available). Preserve: decisions, task state, identifiers. Drop: tool outputs, failed attempts |
| Cost tracking | Accumulate usage fields from every API response in a session struct |
| Prompt caching | Cache system prompt + tool definitions with automatic cache_control. 90% savings on static prefix after first call |
| Context budget | Target 50-100K tokens active context. Compact proactively, not at the limit. Place critical info at start/end |

---

## Sources

- [Token counting - Anthropic Docs](https://platform.claude.com/docs/en/build-with-claude/token-counting)
- [claude-tokenizer crate](https://crates.io/crates/claude-tokenizer/0.3.0)
- [tiktoken-rs crate](https://crates.io/crates/tiktoken-rs)
- [Prompt caching - Anthropic Docs](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
- [Compaction - Anthropic Docs](https://platform.claude.com/docs/en/build-with-claude/compaction)
- [Effective context engineering - Anthropic](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [The Complexity Trap - JetBrains Research / NeurIPS 2025](https://arxiv.org/abs/2508.21433)
- [Context Rot - Chroma Research](https://research.trychroma.com/context-rot)
- [Repository map - Aider](https://aider.chat/docs/repomap.html)
- [Context Is What You Need - arXiv](https://arxiv.org/abs/2509.21361)
- [LLM Chat History Summarization Guide - mem0.ai](https://mem0.ai/blog/llm-chat-history-summarization-guide-2025)
- [Context Window Management Strategies - Maxim](https://www.getmaxim.ai/articles/context-window-management-strategies-for-long-context-ai-agents-and-chatbots/)
- [Understanding LLM Performance Degradation](https://demiliani.com/2025/11/02/understanding-llm-performance-degradation-a-deep-dive-into-context-window-limits/)
- [RULER benchmark - NVIDIA](https://github.com/NVIDIA/RULER)
