# bpmux Bot Session Cost Analysis

Hard numbers on what a bpmux bot participant costs in tokens and dollars.

## Pricing Reference (April 2026)

All prices per million tokens (MTok).

| Model | Input | Output | Cache Write (5min) | Cache Write (1hr) | Cache Read |
|---|---|---|---|---|---|
| Sonnet 4.6 | $3.00 | $15.00 | $3.75 | $6.00 | $0.30 |
| Haiku 4.5 | $1.00 | $5.00 | $1.25 | $2.00 | $0.10 |
| Haiku 3.5 | $0.80 | $4.00 | $1.00 | $1.60 | $0.08 |

Cache read = 10% of base input. Cache write (5min) = 1.25x base. Cache write (1hr) = 2x base.

Batch API gives 50% off everything. Stacks with caching.

Sources: [Anthropic Pricing](https://platform.claude.com/docs/en/about-claude/pricing), [Prompt Caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)

---

## 1. Anatomy of a bpmux Bot API Call

A bpmux bot is not Claude Code. It does not run tools, read files, or enter agentic loops. It reads chat messages and writes a response. This is closer to a Claude.ai chat turn than a Claude Code turn.

### Token budget per API call

| Component | Tokens | Notes |
|---|---|---|
| System prompt | ~2,000 | Bot persona, rules, session context |
| Chat history (20 messages) | ~4,000 | ~200 tokens/message average |
| Bot response | ~200 | Concise chat message, not code generation |
| **Total input** | **~6,000** | |
| **Total output** | **~200** | |

Why 200 tokens per message: chat messages in a pair programming session are short. "Can you explain why you chose that approach?" is 9 tokens. "I think we should extract that into a separate module because the coupling is getting tight" is 19 tokens. Code snippets in chat push it higher. 200 tokens is a generous average that accounts for occasional code blocks.

Why 200 tokens for bot output: the bot is a chat participant, not a code generator. It comments, questions, summarizes. If it generates 500-token responses, something is wrong with the prompt design.

### Cost of a single API call (no caching)

**Sonnet 4.6:**
- Input: 6,000 tokens x $3.00/MTok = $0.018
- Output: 200 tokens x $15.00/MTok = $0.003
- **Total: $0.021 per call**

**Haiku 4.5:**
- Input: 6,000 tokens x $1.00/MTok = $0.006
- Output: 200 tokens x $5.00/MTok = $0.001
- **Total: $0.007 per call**

**Haiku 3.5:**
- Input: 6,000 tokens x $0.80/MTok = $0.0048
- Output: 200 tokens x $4.00/MTok = $0.0008
- **Total: $0.0056 per call**

---

## 2. Cost Per Session

### Assumptions for session modeling

A bpmux bot does not respond to every message. It responds when addressed or when it has something useful to say. Modeling:

| Session length | Human messages | Bot responses | Bot API calls |
|---|---|---|---|
| 30 minutes | ~30 | ~10 | 10 |
| 1 hour | ~60 | ~20 | 20 |
| Full day (6 hrs) | ~360 | ~80 | 80 |

The conversation grows. Early calls are cheap (few messages in context). Late calls are expensive (many messages in context). Modeling the growth:

### 30-minute session (10 bot calls, conversation growing)

Average context per call (messages accumulate):
- Call 1: system(2000) + history(600) = 2,600 input tokens
- Call 5: system(2000) + history(3,000) = 5,000 input tokens
- Call 10: system(2000) + history(6,000) = 8,000 input tokens
- Average input per call: ~5,300 tokens
- Total input: 53,000 tokens
- Total output: 10 x 200 = 2,000 tokens

**Sonnet 4.6 (30 min):**
- Input: 53K x $3.00/MTok = $0.159
- Output: 2K x $15.00/MTok = $0.030
- **Total: $0.19**

**Haiku 4.5 (30 min):**
- Input: 53K x $1.00/MTok = $0.053
- Output: 2K x $5.00/MTok = $0.010
- **Total: $0.063**

### 1-hour session (20 bot calls)

- Average input per call: ~8,000 tokens (conversation is longer)
- Total input: 160,000 tokens
- Total output: 20 x 200 = 4,000 tokens

**Sonnet 4.6 (1 hr):**
- Input: 160K x $3.00/MTok = $0.48
- Output: 4K x $15.00/MTok = $0.06
- **Total: $0.54**

**Haiku 4.5 (1 hr):**
- Input: 160K x $1.00/MTok = $0.16
- Output: 4K x $5.00/MTok = $0.02
- **Total: $0.18**

### Full-day session (80 bot calls over 6 hours)

This assumes a sliding window caps context at some point. Without it, costs explode (see section 5). With a 20-message window:

- Average input per call: ~6,000 tokens (capped by window)
- Total input: 480,000 tokens
- Total output: 80 x 200 = 16,000 tokens

**Sonnet 4.6 (full day):**
- Input: 480K x $3.00/MTok = $1.44
- Output: 16K x $15.00/MTok = $0.24
- **Total: $1.68**

**Haiku 4.5 (full day):**
- Input: 480K x $1.00/MTok = $0.48
- Output: 16K x $5.00/MTok = $0.08
- **Total: $0.56**

### Summary table

| Session | Sonnet 4.6 | Haiku 4.5 | Haiku 3.5 |
|---|---|---|---|
| 30 minutes | $0.19 | $0.063 | $0.050 |
| 1 hour | $0.54 | $0.18 | $0.14 |
| Full day (6 hrs) | $1.68 | $0.56 | $0.45 |

---

## 3. Prompt Caching Impact

### Setup

- System prompt: 2,000 tokens (cacheable)
- Cache strategy: 1-hour TTL (2x write cost, 0.1x read cost)
- Session: 1 hour, 20 bot calls

### Without caching (Sonnet 4.6)

System prompt input cost: 20 calls x 2,000 tokens x $3.00/MTok = $0.12

### With 1-hour caching (Sonnet 4.6)

- First call (cache write): 2,000 tokens x $6.00/MTok = $0.012
- Remaining 19 calls (cache read): 19 x 2,000 tokens x $0.30/MTok = $0.0114
- **Total system prompt cost: $0.023**

**Savings on system prompt: $0.12 - $0.023 = $0.097 (81% reduction)**

### In context of total session cost

The system prompt is only ~25% of input tokens (2,000 out of ~8,000 average). Caching it saves ~$0.10 on a $0.54 session. That is an 18% total cost reduction.

If we could also cache the conversation prefix (messages that don't change between calls), the savings compound. Each call re-sends all previous messages. If the first 15 messages are cached on call 20:

- 15 messages x 200 tokens = 3,000 tokens cached
- Cache read: 3,000 x $0.30/MTok = $0.0009 vs full input: 3,000 x $3.00/MTok = $0.009
- Saves $0.008 per call, roughly $0.08 over the back half of a 1-hour session

**Total caching benefit for 1-hour session (Sonnet): ~$0.18 saved, reducing $0.54 to ~$0.36 (33% reduction)**

---

## 4. Sliding Window vs Full Context

### The math on context growth

If the bot sends ALL messages (no window), input tokens grow linearly. In a 1-hour session with 60 total messages:

| Call # | Messages in context | Input tokens | Cumulative input |
|---|---|---|---|
| 1 | 3 | 2,600 | 2,600 |
| 5 | 15 | 5,000 | 19,000 |
| 10 | 30 | 8,000 | 53,000 |
| 15 | 45 | 11,000 | 95,000 |
| 20 | 60 | 14,000 | 148,000 |
| **Total** | | | **~317,600** |

vs a 20-message sliding window:

| Call # | Messages in context | Input tokens | Cumulative input |
|---|---|---|---|
| 1 | 3 | 2,600 | 2,600 |
| 5 | 15 | 5,000 | 19,000 |
| 10 | 20 (capped) | 6,000 | 47,000 |
| 15 | 20 (capped) | 6,000 | 77,000 |
| 20 | 20 (capped) | 6,000 | 107,000 |
| **Total** | | | **~160,000** |

### Cost comparison (Sonnet 4.6, 1-hour session)

| Strategy | Total input tokens | Input cost | Output cost | Total |
|---|---|---|---|---|
| All messages | 317,600 | $0.95 | $0.06 | $1.01 |
| Last 50 messages | ~240,000 | $0.72 | $0.06 | $0.78 |
| Last 20 messages | ~160,000 | $0.48 | $0.06 | $0.54 |
| Last 10 messages | ~120,000 | $0.36 | $0.06 | $0.42 |

### The inflection point

For a full-day session (360 messages, 80 bot calls), full context is catastrophic:

- Full context average input: ~36,000 tokens/call
- Total input: 2.88M tokens
- Sonnet cost: **$8.64 input + $0.24 output = $8.88**

vs 20-message window: **$1.68**

Full context costs 5.3x more than a 20-message window in a full-day session. The window pays for itself immediately.

### Where the window hurts

A 10-message window loses conversational coherence. The bot forgets decisions made 15 minutes ago. A 20-message window covers roughly 10-15 minutes of conversation -- enough for most immediate context. For longer-range context, a summary/compression strategy (see section 8) is cheaper than expanding the window.

---

## 5. Comparison to Claude Code

### What Claude Code actually costs

Real-world data from multiple sources:
- Average: ~$6/developer/day
- 90th percentile: ~$12/day
- Monthly: $100-200/developer with Sonnet 4.6
- A single "edit this file" command: 50,000-150,000 tokens (one API call)
- Turn 1: ~15,000 tokens. Turn 15: ~100,000 tokens. Turn 30: ~167,000 tokens.
- System prompt alone: ~14,328 tokens

Sources: [Claude Code Costs](https://code.claude.com/docs/en/costs), [Faros Token Limits](https://www.faros.ai/blog/claude-code-token-limits)

### Side-by-side

| Metric | Claude Code (1 hr) | bpmux bot Sonnet (1 hr) | bpmux bot Haiku (1 hr) |
|---|---|---|---|
| API calls | ~15-30 (agentic loops) | ~20 (chat responses) |  ~20 |
| Tokens per call | 15K-167K | ~6K | ~6K |
| Total input tokens | ~1-2M | ~160K | ~160K |
| Total output tokens | ~50-100K | ~4K | ~4K |
| System prompt size | ~14,328 tokens | ~2,000 tokens | ~2,000 tokens |
| Cost per hour | $1-6 | $0.54 | $0.18 |
| Cost per day | $6-12 | $1.68 | $0.56 |

**The bpmux bot is 6-20x cheaper per hour than Claude Code.**

This makes sense: Claude Code runs tools, reads files, enters multi-step agentic loops, and generates large code blocks. The bpmux bot reads chat and writes short responses. Fundamentally different workloads.

### The real comparison

The question is not "bot vs Claude Code" -- they do different things. The question is: "Does adding a bot to a bpmux session cost more than the value it provides?"

At $0.18/hour on Haiku, the bot costs less than a cup of coffee per full workday. The bar for "providing value" is absurdly low.

---

## 6. The $20/Month Subscriber Concern

### What Pro subscribers get

- $20/month for Claude Pro
- ~45 messages per 5-hour window on claude.ai
- Claude Code on Pro: ~44,000 tokens per 5-hour window
- No direct API access -- subscribers use claude.ai or Claude Code, not the API

### The fundamental problem

bpmux bots use the API, not claude.ai. A $20/month subscriber does not get API access. They would need:

1. **API credits** -- separate from the subscription. Pay-as-you-go.
2. **Max plan** ($100/month for Max5, $200/month for Max20) -- more generous Claude Code limits but still not API access for third-party tools.

### If bpmux used subscriber tokens (hypothetical)

If Anthropic allowed third-party tools to use subscriber quotas (they don't), here is what fits:

Pro gets ~44,000 tokens per 5-hour window. A bpmux bot call uses ~6,000 input + 200 output = 6,200 tokens. That is ~7 bot responses per window. Over a workday (roughly 2 windows), that is 14 bot responses. Barely functional.

Max5 (~88,000 tokens/window) gives ~14 bot responses per window, ~28 per day. Usable but tight.

### Realistic path for subscribers

A subscriber who wants a bpmux bot pays API rates on top of their subscription:
- Haiku 4.5 full day: $0.56
- Haiku 4.5 full month (22 working days): $12.32
- Sonnet 4.6 full month: $36.96

**A Haiku bot costs $12/month on top of the subscription. Sonnet costs $37/month.** Both are viable for individual developers. The concern is not cost -- it is the friction of setting up API billing separately.

---

## 7. Cost Optimization Strategies

### Strategy 1: Model routing (biggest impact)

Route by message complexity:
- **Haiku 4.5 (default):** Simple observations, acknowledgments, short questions. 80% of responses.
- **Sonnet 4.6 (escalation):** Architecture discussions, complex trade-off analysis, debugging assistance. 20% of responses.

**Blended cost for 1-hour session:**
- 16 Haiku calls: 16 x $0.007 = $0.112
- 4 Sonnet calls: 4 x $0.021 = $0.084
- **Total: $0.196** vs $0.54 pure Sonnet (64% savings)

The routing trigger can be simple: message length, presence of code blocks, explicit @bot-sonnet addressing.

### Strategy 2: Response length limits (moderate impact)

Output tokens are 5x more expensive than input tokens (Sonnet). Capping bot responses at 150 tokens instead of 200 saves:
- Per call: 50 tokens x $15/MTok = $0.00075
- Per 1-hour session: 20 x $0.00075 = $0.015

Small in absolute terms but enforces good design. Chatty bots are bad bots regardless of cost.

### Strategy 3: Prompt caching (moderate impact)

As calculated in section 3: ~33% reduction on system prompt + conversation prefix. Worth implementing but not transformative at these price points.

### Strategy 4: Summarization triggers (high impact at scale)

When conversation exceeds N messages, summarize older messages into a compressed context block:
- 50 messages at 200 tokens each = 10,000 tokens
- Summary of those 50 messages: ~500 tokens
- Savings per subsequent call: 9,500 tokens input

For a full-day session on Sonnet:
- Without summarization (20-msg window): $1.68
- With summarization (summary + 20-msg window): ~$1.80 (summary generation costs offset savings)

Summarization does not save much money with a sliding window already in place. Its value is context quality, not cost. The bot remembers decisions from 2 hours ago without paying for 2 hours of raw messages.

### Strategy 5: Batch API (situational)

50% off everything, but responses are asynchronous (up to 24 hours). Not viable for real-time chat. Could work for end-of-session summaries or daily digests.

### Strategy 6: Skip unnecessary calls

The cheapest API call is the one you don't make. If the bot has nothing useful to add, it should stay silent. A well-tuned bot that responds to 10 messages/hour instead of 20 cuts costs in half.

### Ranked by impact

| Strategy | Savings | Complexity | Notes |
|---|---|---|---|
| Model routing | 50-65% | Medium | Biggest single lever |
| Skip unnecessary calls | 30-50% | Low | Prompt engineering |
| Prompt caching | 18-33% | Low | Easy to implement |
| Summarization | 5-10% cost, high context quality | Medium | Value is in quality, not cost |
| Response length caps | 3-5% | Low | Good discipline regardless |
| Batch API | 50% | N/A | Not viable for real-time |

---

## 8. Bottom Line

### The bpmux bot is cheap

| Scenario | Haiku 4.5 | Sonnet 4.6 | Routed (80/20) |
|---|---|---|---|
| Per API call | $0.007 | $0.021 | ~$0.010 |
| 30-minute session | $0.063 | $0.19 | ~$0.10 |
| 1-hour session | $0.18 | $0.54 | ~$0.20 |
| Full workday (6 hrs) | $0.56 | $1.68 | ~$0.70 |
| Full month (22 days) | $12.32 | $36.96 | ~$15.40 |

### Comparison context

- Claude Code costs $6-12/developer/day. A bpmux Haiku bot costs $0.56/day.
- Aider costs $60-80/month at heavy use. A bpmux Haiku bot costs $12/month.
- A Claude Pro subscription costs $20/month. A bpmux Haiku bot costs $12/month on the API.
- A developer's hourly rate is $75-200. A bpmux bot costs less per day than a developer costs per minute.

### What this means for design decisions

1. **Cost is not a blocking concern.** Even Sonnet at full price is under $2/day. This is noise in any professional context.
2. **Model routing is the only optimization that matters at launch.** Haiku-default with Sonnet-escalation gives 80% of the quality at 35% of the cost.
3. **Sliding window is mandatory.** Not for cost (though it helps) but because full context is architecturally wrong. A 20-message window is the right default.
4. **Subscribers need API keys.** There is no way around this. The bot runs on the API, not on subscriber quotas. The friction is in setup, not in cost.
5. **Caching is worth implementing but not urgent.** At these price points, the engineering time to implement caching exceeds the dollar savings for months.
6. **The real cost is not the bot.** It is the Claude Code sessions that developers run alongside it. The bot is a rounding error on the total AI spend.
