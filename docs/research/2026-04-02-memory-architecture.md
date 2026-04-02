# Memory Architecture: Short-Term + Long-Term for bpmux

*Researched April 2, 2026. Exploring tiered memory as a core bpmux capability.*

---

## The Problem

bpmux v1 uses recency as a proxy for relevance: `read_messages()` returns unread messages (short-term) or the last N messages (fallback). This works for "what just happened?" but fails for "what did we decide about auth three weeks ago?" Old decisions that are still relevant get buried. New noise that isn't relevant gets surfaced.

The "stale CLAUDE.md" problem (ETH Zurich AGENTbench, 2026) shows this isn't theoretical — outdated context actively degrades AI performance. And the reverse is true too: missing relevant old context means the AI rediscovers or contradicts prior decisions.

## The Research-Backed Solution: Tiered Memory

### MemGPT (Packer et al., 2023; peer-reviewed at ICLR 2024)

MemGPT treats conversation history as a tiered memory system:
- **Main context** ("RAM") — the current working context, bounded by the context window
- **Archival storage** ("disk") — the full history, searchable via semantic retrieval

The AI pages relevant history into its working context when needed, like an operating system pages memory. MemGPT agents significantly outperform fixed-context-window agents on multi-session tasks.

**Source:** [MemGPT: Towards LLMs as Operating Systems (arXiv:2310.08560)](https://arxiv.org/abs/2310.08560)

### Hybrid Scoring: RIR (Recency + Importance + Relevance)

Research on personalized recommendation (ARAG, 2025) compared three retrieval strategies:
- **Recency** — recent messages. Cheap, fast. Wins when the task is about "what just happened."
- **Relevance** — semantic similarity. Wins for factual retrieval and precise reference lookups.
- **Agentic RAG** — hybrid scoring across multiple dimensions. Significantly outperformed both pure strategies.

The emerging consensus: score context along multiple vectors (recency, importance, relevance) and filter dynamically. Pure recency misses important old decisions. Pure relevance surfaces superseded information without temporal awareness.

**Source:** [ARAG — Adaptive RAG (arXiv:2506.21931)](https://arxiv.org/abs/2506.21931)

### The "Lost in the Middle" Problem

Databricks found that model performance degrades after certain context sizes (32k tokens for Llama, 64k for GPT-4). Even when you *can* fit everything in context, models struggle with information in the middle of long sequences. Selective retrieval beats context-stuffing.

**Source:** [Databricks Long-Context RAG Study](https://www.databricks.com/blog/long-context-rag-performance-llms)

## Proposed Architecture for bpmux

### Two Memory Systems, One Log

The JSONL message log remains the single source of truth. Two access patterns serve two different needs:

**Short-term memory (`read_messages`):**
- Server-side read cursor — returns unread since last call
- Bounded window fallback (last 50 for new clients)
- Answers: "What just happened? What's new?"
- Already designed in v1 spec

**Long-term memory (`search_messages`):**
- RAG over the full message log — semantic search by query
- Returns relevant messages regardless of when they were sent
- Answers: "What did we decide about X? What's the context for Y?"
- Future feature — v1.x or v2

### The Stale Context Risk

RAG has a documented failure mode: old decisions surface with high semantic similarity even when they've been superseded. A decision from week 1 about using library X scores as highly as the decision from week 3 to switch to library Y — both are about the same topic.

Mitigations from the research (practitioner experience, not yet rigorously benchmarked):
- **Temporal decay multipliers** — older messages score lower, all else being equal
- **Supersession markers** — when a new decision contradicts an old one, mark the old one as superseded
- **Hybrid RIR scoring** — combine recency, importance, and relevance rather than pure semantic similarity

bpmux's existing age decay concept (designed for visual rendering in v1.x) could serve double duty: decay the retrieval score as well as the visual prominence. Pheromone evaporation applied to both the human view and the AI's retrieval.

### Token Efficiency

RAG achieves 50-200x reduction in input tokens vs full context dumping (Microsoft, MindStudio, multiple sources). For bpmux: if you have 100k tokens of conversation history and RAG retrieves 5k tokens of relevant context, that's 95% fewer tokens consumed. This directly addresses the $20/month subscriber concern.

**Sources:** [Microsoft Context-Aware RAG](https://techcommunity.microsoft.com/blog/azure-ai-foundry-blog/context-aware-rag-system-with-azure-ai-search-to-cut-token-costs-and-boost-accur/4456810) | [Adaptive Context Compression (EMNLP 2025)](https://aclanthology.org/2025.findings-emnlp.1307.pdf)

### In-Process Rust Implementation

This can run entirely in-process — no external vector DB:
- **Embeddings:** `fastembed-rs` or Candle with all-MiniLM-L6-v2 (~80MB model, runs on CPU)
- **Vector search:** VectorLite (in-memory HNSW index, sub-millisecond search) or LanceDB (columnar, in-process)
- **Index update:** On each new message, compute embedding and add to index. Cheap and incremental.

Single binary preserved. No network calls. No external services.

### MCP Tools (proposed)

```
read_messages()              — short-term: what's new since last call
read_messages(last: N)       — short-term: recent window
search_messages(query: "...")  — long-term: what's relevant to this topic
```

The AI uses `read_messages` for ongoing awareness and `search_messages` when it needs to recall prior context. Both return messages from the same log. Both respect trust levels and security wrapping.

## Connection to Theoretical Framework

### Stigmergy
Short-term memory is following fresh pheromone trails (recent activity). Long-term memory is searching for strong trails (reinforced decisions). Both are forms of reading traces from the shared environment.

### Cognitive / Intent Debt (Storey 2026)
Short-term memory prevents cognitive debt (what just happened is visible). Long-term memory prevents intent debt (why decisions were made is retrievable). Together they address both debts.

### Bitter Lesson (Sutton 2019)
`search_messages` is search — a general method that scales with computation. It doesn't encode heuristics about what's relevant. It finds relevance through the embedding space. Sutton-aligned.

### Ubuntu / Collective Memory
When two nodes connect and merge their logs, the combined long-term memory contains both teams' accumulated reasoning. Each node's AI "is because the others were" — it can search for reasoning it never participated in.

## Session Transitions: What Happens When You Switch Sessions?

When an AI session ends and a new one begins, the new session has zero context. This is the restart problem bpmux was originally designed to solve. The memory architecture determines how much context the new session recovers.

### OpenClaw's Three-Tier Model (Practitioner Implementation)

OpenClaw (open-source AI assistant, Peter Steinberger) implements the most mature version of tiered memory observed:

- **Tier 1 (always loaded):** ~100 lines of curated essentials — MEMORY.md. Always in context every conversation. What matters most.
- **Tier 2 (daily context):** Date-stamped files. Today and yesterday load automatically. Recent activity.
- **Tier 3 (deep knowledge):** Structured by topic (people, projects, decisions). Searched via vector embeddings on demand.

A "librarian" skill promotes durable knowledge upward: daily observations become structured knowledge → structured knowledge gets summarized into MEMORY.md. Knowledge flows up the tiers as it proves durable.

**Source:** [OpenClaw Config](https://github.com/TechNickAI/openclaw-config) | [OpenClaw Architecture Deep Dive](https://openclawdesktop.com/blog/openclaw-architecture-deep-dive.html)

### Proposed bpmux Session Transition Model

Mapping OpenClaw's tiers to bpmux's architecture:

| Tier | Content | When Loaded | bpmux Implementation |
|---|---|---|---|
| Tier 1 — Essential | Active decisions, current project state | Always included in every `read_messages` response | Auto-generated summary of pinned/reinforced messages (v2 — close to deferred "shared notes") |
| Tier 2 — Recent | Messages since last call, or last 50 for new sessions | Default `read_messages` behavior | Already designed (v1 — read cursor + bounded window) |
| Tier 3 — Deep | Full message history, searchable by topic | On demand via `search_messages(query)` | RAG over JSONL log (v1.x/v2) |

**New session experience:**
1. AI connects via MCP, calls `read_messages()` → gets Tier 2 (recent 50 messages)
2. If Tier 1 is implemented, essential context is prepended automatically — the AI knows the key decisions without searching
3. During work, AI calls `search_messages(query)` when it needs older context → Tier 3

**The knowledge promotion question:** OpenClaw's "librarian" that promotes knowledge upward is a valuable pattern. For bpmux, this could be: messages that are referenced frequently in `search_messages` results get promoted to Tier 1 (auto-surfaced). Or: humans explicitly pin important decisions. The promotion mechanism is a v2 design question.

**What v1 handles:** Tier 2 only. The read cursor and bounded window give new sessions recent context. This is sufficient for dogfooding. If it proves insufficient (the "I keep having to tell my AI what we decided" signal), Tier 1 and Tier 3 become the next stories.

## What This Means for the Spec

**v1:** Unchanged. `read_messages` with cursor and bounded window. Simple, shipped, dogfooded.

**v1.x or v2:** Add `search_messages` backed by in-process RAG. This is when the memory architecture becomes tiered. The decision to promote this depends on dogfooding: does recency-based context prove insufficient?

**The hypothesis to test:** Is recency a good enough proxy for relevance in practice, or do developers and AIs frequently need to recall old decisions? Dogfooding will answer this. If the answer is "I keep having to tell my AI what we decided two weeks ago," RAG becomes the next story.

## Sources

- [MemGPT (2023)](https://arxiv.org/abs/2310.08560) — peer-reviewed
- [ARAG (2025)](https://arxiv.org/abs/2506.21931) — peer-reviewed
- [Databricks Long-Context Study](https://www.databricks.com/blog/long-context-rag-performance-llms) — industry benchmark
- [Microsoft Context-Aware RAG](https://techcommunity.microsoft.com/blog/azure-ai-foundry-blog/context-aware-rag-system-with-azure-ai-search-to-cut-token-costs-and-boost-accur/4456810) — industry benchmark
- [EMNLP 2025 Adaptive Context Compression](https://aclanthology.org/2025.findings-emnlp.1307.pdf) — peer-reviewed
- [ETH Zurich AGENTbench (2026)](https://arxiv.org/html/2602.11988v1) — peer-reviewed
- [fastembed-rs](https://github.com/Anush008/fastembed-rs) — Rust embeddings
- [VectorLite](https://github.com/mmailhos/vectorlite) — in-process vector DB
