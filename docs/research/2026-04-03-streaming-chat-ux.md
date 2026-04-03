# Streaming Chat UX: How AI Participants Deliver Responses in Real-Time

Research into the user experience of watching an AI respond in group chat. Focused on what feels natural, what annoys people, and what production systems actually do.

---

## 1. Token Streaming in Chat Platforms

### How Discord Bots Handle It

Discord has **no native streaming API for bot messages**. A 2023 feature request for a streaming endpoint (`POST /channels/{channel.id}/messages/live`) was rejected by Discord's team with no plans to implement it.

The universal workaround: **edit-in-place**. Bots send an initial message, then repeatedly PATCH it with accumulated tokens. The constraint is Discord's rate limit -- roughly one edit per 1-2 seconds per channel. Production implementations (e.g., CoPaw's DiscordDraftStream) throttle edits to ~1.2 seconds between updates and send typing indicators every 10 seconds during generation.

A commenter on the rejected proposal offered a pragmatic alternative: train the AI to send multiple shorter messages instead of streaming one long one, since that is how humans actually use Discord.

- Source: https://github.com/discord/discord-api-docs/discussions/6310
- Source: https://github.com/agentscope-ai/CoPaw/issues/1296

### How Slack Bots Handle It

Slack's `chat.update` API imposes a hard rate limit of **one update per second per channel**. Additionally, a message that is currently streaming cannot be edited -- you must wait for one edit to complete before sending another. This makes token-level streaming impossible on Slack. The practical pattern is buffering chunks and updating the message roughly once per second.

- Source: https://docs.slack.dev/reference/methods/chat.update/
- Source: https://docs.slack.dev/apis/web-api/rate-limits/

### What ChatGPT and Claude Do

Both use **Server-Sent Events (SSE)** over HTTP. The API streams token-by-token, with each chunk wrapped in an SSE `data:` field containing a JSON delta. On the client side, tokens are rendered at ~16ms intervals (60fps throttling). The effect: text appears to "type itself" at roughly reading speed.

Claude's web interface uses DOM diffing (morphdom or similar) rather than naive innerHTML replacement, applying minimal patches and animating only genuinely new nodes. This prevents layout jitter during streaming.

- Source: https://developers.openai.com/api/docs/guides/streaming-responses
- Source: https://platform.claude.com/docs/en/agent-sdk/streaming-output
- Source: https://michaellivs.com/blog/reverse-engineering-claude-generative-ui/

### The Fundamental Tension

In a **web app you control** (like bpmux), you can stream token-by-token over WebSocket with no rate limiting. In **third-party platforms** (Discord, Slack), you are constrained to periodic message edits. The UX gap is significant: ChatGPT feels fluid; a Discord bot feels like it is updating in jerky chunks.

---

## 2. The Typing Indicator Problem

### What Research Says

A 2025 study in the International Journal of Human-Computer Interaction found that typing indicators during AI processing **mitigate the negative effects of longer response times** by increasing perceived social presence. Users feel the AI is "working on it" rather than ignoring them.

However, a separate 2025 study (tested on 40 participants with three visual conditions: blank screen, animated dots, and character-by-character typing) found **no significant difference in perceived usability** across conditions. This challenges the assumption that humanizing the wait automatically improves UX.

- Source: https://www.tandfonline.com/doi/full/10.1080/10447318.2025.2508915
- Source: https://userexperience.org/blog/2025/01/03/chatbot-delay/

### The Artificial Delay Debate

Should an AI that can respond in 200ms wait longer to seem more "human"?

Research is mixed. Gnewuch et al. (2018) found dynamic delays (50ms per character of the response) increased perceived humanness and satisfaction. But context matters: in customer service scenarios where speed is expected, artificial delays hurt satisfaction. Moon (1999) found medium response times led to higher persuasiveness than either instant or slow.

The emerging view: as users become more experienced with AI, they may prefer AI-native interaction patterns (fast, systematic) over faked human-like timing. The CASA (Computers Are Social Actors) framework may be losing relevance for experienced AI users.

- Source: https://link.springer.com/article/10.1007/s12599-022-00755-x
- Source: https://www.researchgate.net/publication/324949980

### Practical Recommendation for bpmux

In a P2P group chat where participants know they are talking to an AI, faking human-like delays is probably counterproductive. Instead:

1. Show a typing/processing indicator immediately when the AI begins generating
2. Start streaming tokens as soon as they are available
3. Do not add artificial delays -- the natural latency of LLM inference (1-3 seconds to first token) provides enough "thinking time" to feel natural

---

## 3. Latency Perception Thresholds

### Nielsen's Three Limits (Still Canonical)

Jakob Nielsen's 1993 response time research remains the foundation:

| Threshold | Perception | Implication |
|-----------|-----------|-------------|
| **0.1s** | Instantaneous | No feedback needed. User feels they caused the result directly. |
| **1.0s** | Flow preserved | User notices delay but stays focused. Feels sluggish but tolerable. |
| **10s** | Attention lost | User starts thinking about other things. Needs progress indicator. |

- Source: https://www.nngroup.com/articles/response-times-3-important-limits/

### Applied to AI Responses

Most LLM API calls take 1-5 seconds to first token, with full responses taking 5-30 seconds. This means:

- **Time to first token (TTFT)** is the critical metric. If the user sees *something* within 1 second, flow is preserved. Streaming achieves this even when the full response takes 20 seconds.
- **Without streaming**, the user stares at nothing for 5-30 seconds, well past the 10-second attention threshold. This is why every major AI interface streams.
- **With streaming**, the experience transforms from "waiting for a response" to "watching the AI think," which users evaluate differently. The 10-second limit applies to *dead silence*, not to actively arriving content.

Nielsen's own assessment of current AI: most systems fail to meet even the least demanding threshold, making the experience unpleasant. Streaming is the primary mitigation.

- Source: https://www.nngroup.com/articles/powers-of-10-time-scales-in-ux/
- Source: https://jakobnielsenphd.substack.com/p/time-scale-ux

### For bpmux

Users will judge responsiveness by **time to first visible token**, not time to complete response. Target: first token visible within 1 second of the AI being invoked. If the LLM backend is slower than that, show a status indicator (processing/thinking) within 100ms.

---

## 4. Multi-Turn Conversation Pacing

### The Group Chat Challenge

In a group chat with 2 humans and 1 AI, timing is harder than in 1:1 chat. The AI must decide:

- **When to respond**: Immediately when mentioned? After a pause in conversation? Only when explicitly asked?
- **How to handle overlapping messages**: If both humans are typing, should the AI wait for both to finish?
- **How to avoid dominating**: AI responses tend to be longer and more complete than human messages, which can overwhelm the conversation.

### What Production Systems Do

**Microsoft Teams bots**: In group conversations, bots must be explicitly @mentioned to respond. This is a deliberate design choice -- it prevents the bot from jumping into every exchange.

**Poe group chat** (launched November 2025): Supports up to 200 users with 200+ AI models in a single conversation. AI bots only respond when explicitly invoked by a user selecting a model. Chat history syncs in real-time across devices. Quora spent six months engineering the system to handle concurrent interactions at scale.

**HighLevel Conversation AI**: Uses configurable wait times of 5-20 seconds in one-on-one chats before the bot responds, giving the human time to send follow-up messages. Too short and the bot interrupts; too long and it feels unresponsive.

- Source: https://learn.microsoft.com/en-us/microsoftteams/platform/bots/how-to/conversations/channel-and-group-conversations
- Source: https://techcrunch.com/2025/11/18/poes-ai-app-now-supports-group-chats-across-ai-models/
- Source: https://help.gohighlevel.com/support/solutions/articles/155000004415

### Pacing Strategies

1. **Explicit invocation only** (safest): AI responds only when @mentioned or through a slash command. Prevents interruption. Used by most production bots.
2. **Debounced response**: Wait N seconds after the last human message before responding. Prevents the AI from jumping in while humans are mid-thought. The risk: feels sluggish.
3. **Immediate on mention, silent otherwise**: The AI is present but quiet unless addressed. Good for "AI as participant" rather than "AI as moderator."
4. **Message spacing**: Instead of one long response, break it into 2-3 shorter messages with slight pauses between them. Mimics human conversation cadence.

### Practical Recommendation for bpmux

Given bpmux is for AI pair programming, explicit invocation (option 1 or 3) is the right default. The AI should respond when addressed and stay quiet otherwise. For multi-message exchanges, a short debounce (2-3 seconds) after the last human message prevents the AI from cutting in mid-thought.

---

## 5. Message Chunking Strategies

### What Chunk Size Feels Natural?

There is a spectrum from "token-by-token" to "full message at once":

| Strategy | Chunk Size | Feel | Used By |
|----------|-----------|------|---------|
| Per-token | 1-5 chars | Fluid, typewriter effect | ChatGPT, Claude web UI |
| Per-word | 3-8 chars | Slightly smoother than per-token | Some custom implementations |
| Per-sentence | 20-100 chars | Choppy, feels like buffered updates | Some Discord/Slack bots |
| Per-paragraph | Variable | Very choppy, feels like batch processing | Legacy chat bots |
| Full message | Complete | Long wait then wall of text | Pre-streaming era |

### Production Evidence

OpenAI streams token-by-token (1-5 characters per SSE event). The recommended client-side approach: render every token immediately with 16ms throttling for 60fps smoothness. Do not batch tokens unless rendering performance degrades.

For platforms with rate limits (Discord, Slack), the forced chunking creates a noticeably worse experience. The CoPaw Discord implementation buffers tokens and flushes to Discord every ~1.2 seconds, which means the message visibly "jumps" forward in chunks rather than flowing smoothly.

- Source: https://makeaihq.com/guides/cluster/streaming-responses-real-time-ux-chatgpt
- Source: https://developers.openai.com/api/docs/guides/streaming-responses

### Protocol Choice: SSE vs WebSocket

For bpmux specifically (P2P, bidirectional):

| | SSE | WebSocket |
|---|-----|-----------|
| Direction | Server-to-client only | Bidirectional |
| Complexity | Simple, standard HTTP | More complex handshake |
| AI token streaming | Works well | Works well |
| User sending messages | Needs separate HTTP POST | Same connection |
| P2P architecture | Poor fit (requires server) | Natural fit |
| Binary data | Not supported | Supported |
| Reconnection | Built-in | Manual |

For bpmux, which already uses WebSocket for P2P messaging, streaming tokens over the same WebSocket connection is the obvious choice. SSE is better for client-server architectures where the AI provider pushes tokens to a web frontend.

The Vercel AI SDK recently deprecated its HTTP+SSE transport in favor of a pluggable interface, reflecting the industry shift toward WebSocket for more complex AI applications.

- Source: https://ably.com/blog/websockets-vs-sse
- Source: https://rxdb.info/articles/websockets-sse-polling-webrtc-webtransport.html

### Practical Recommendation for bpmux

Stream token-by-token over WebSocket. Each token arrives as a small JSON message (type: "token", content: "Hello"). The TUI renders tokens as they arrive with no artificial batching. The natural latency of P2P message delivery (milliseconds) adds just enough smoothing.

---

## 6. Error States in Chat

### Failure Modes

AI responses can fail in several ways, each requiring different UX:

| Failure | When | What User Sees |
|---------|------|---------------|
| **API timeout** | LLM takes too long | Message stops mid-sentence or never starts |
| **Mid-stream disconnect** | Network drops during generation | Partial response frozen in place |
| **Rate limit (429)** | Too many requests | No response at all |
| **Model refusal** | Content policy triggered | Partial or empty response |
| **Context overflow** | Conversation too long | Error before generation starts |

### UX Patterns for Error Recovery

The AI Error States pattern (uxpatterns.dev) identifies five required components:

1. **Error summary**: Plain language, not technical jargon
2. **Context detail**: Whether the issue is policy, limits, network, or availability
3. **Recovery action**: Retry, shorten input, switch model, or continue manually
4. **Preserved input**: Keep the original prompt visible so users can retry
5. **Escalation path**: Fallback workflow if retry fails

Key anti-patterns to avoid:
- Collapsing all failures into a generic "something went wrong" message
- Hiding system state so users cannot tell if the AI is waiting, streaming, or failed
- Treating AI failures like form validation errors (they need specialized recovery paths)

- Source: https://uxpatterns.dev/patterns/ai-intelligence/ai-error-states
- Source: https://uxpatterns.dev/patterns/ai-intelligence/ai-chat

### Mid-Stream Failure Specifically

OpenAI's streaming API may return HTTP 200 initially, then send an error as a JSON object mid-stream. Detection heuristic: if no tokens arrive for 15 seconds mid-stream, something is wrong. At that point, you have partial results that should be preserved.

The recommended pattern: show the partial response with a clear indicator that generation was interrupted, and offer a "Continue" or "Retry" action. Do not silently discard partial content.

- Source: https://ofox.ai/blog/ai-api-error-handling-troubleshooting-guide-2026/

### Practical Recommendation for bpmux

In a P2P chat, errors should appear as system messages in the conversation (not modal dialogs or toasts). The partial response should remain visible with a visual marker (e.g., "[interrupted]" suffix or different formatting). The user should be able to retry with a simple command. Different failure modes should produce different messages:

- Timeout: "AI response timed out after 30s. /retry to try again."
- Rate limit: "Rate limited. Will retry in {N} seconds."
- Refusal: Show what the model returned, clearly marked.
- Disconnect: Show partial response + "[connection lost]"

---

## 7. Real Implementations Compared

### ChatGPT (OpenAI)

- **Protocol**: SSE over HTTP
- **Chunking**: Token-by-token (1-5 chars per event)
- **Client rendering**: 16ms throttle (60fps), DOM updates per token
- **Typing indicator**: None -- text starts appearing immediately (TTFT ~0.5-2s)
- **Error handling**: Mid-stream errors sent as JSON in the SSE stream
- **Group/shared**: ChatGPT has experimented with shared conversations in some markets (Asia) but this is not a core feature

### Claude (Anthropic)

- **Protocol**: SSE over HTTP
- **Chunking**: Token-by-token via `content_block_delta` events
- **Client rendering**: DOM diffing (morphdom-style) for minimal patches
- **Streaming structure**: Progressive rendering -- HTML structure builds visually, styles apply, cards and tables appear piece by piece
- **Typing indicator**: Animated dots appear briefly before first token
- **Error handling**: Structured event types distinguish content from errors

### Poe (Quora)

- **Architecture**: API orchestrator -- forwards queries to selected model APIs
- **Group chat**: Up to 200 users + multiple AI models in one conversation (launched Nov 2025)
- **Streaming**: Each AI model streams independently; responses appear in real-time
- **Cross-device**: Chat history syncs in real-time across all devices
- **Invocation model**: Users explicitly select which AI model responds (no automatic triggering)

- Source: https://techcrunch.com/2025/11/18/poes-ai-app-now-supports-group-chats-across-ai-models/
- Source: https://poe.com/blog/introducing-group-chat-for-all-poe-users

### Discord AI Bots (Various)

- **Protocol**: REST API (edit message in place)
- **Chunking**: Forced to ~1.2s intervals by rate limits
- **Typing indicator**: Bots send explicit typing indicator via API every 10s
- **Feel**: Noticeably choppier than web-native AI interfaces
- **Workaround**: Some bots send multiple shorter messages instead of streaming one long one

### Slack AI Bots

- **Protocol**: REST API (chat.update)
- **Chunking**: Limited to 1 update per second per channel
- **Constraint**: Cannot edit a message that is currently being updated
- **Feel**: Similar choppiness to Discord; most Slack AI bots just send complete messages

---

## Summary: What Matters for bpmux

### The Non-Negotiables

1. **Stream token-by-token over WebSocket**. bpmux controls the transport, so there is no reason to accept the chunky experience of platform-constrained bots.
2. **Show first token within 1 second**. This is the threshold between "feels responsive" and "feels broken." If the LLM backend is slower, show a processing indicator within 100ms.
3. **Explicit invocation in group chat**. The AI responds when addressed, not on every message. This prevents the AI from dominating the conversation.
4. **Distinct error states**. Different failures get different messages. Partial responses are preserved, not discarded.

### The Judgment Calls

1. **Artificial delays**: Probably not. In a developer tool where participants know they are talking to an AI, speed is a feature. The natural 1-3s TTFT of LLM inference provides adequate "thinking time."
2. **Typing indicator vs immediate streaming**: Show a brief status ("thinking...") that transitions to streaming text. The status covers the TTFT gap; streaming covers the rest.
3. **Message length**: The AI should prefer concise responses in chat. Long responses should be split across messages or offered as expandable content, not dumped as walls of text.
4. **Response cancellation**: Users should be able to stop a streaming response mid-generation. This respects both bandwidth and attention.

### The Architecture

```
User types @ai question
  -> bpmux sends to AI service over WebSocket/HTTP
  -> AI service calls LLM API (SSE stream)
  -> AI service forwards tokens over WebSocket to all peers
  -> Each peer's TUI renders tokens as they arrive
  -> On completion, message is finalized in the log
```

The key insight: bpmux sits between the rate-limited world of third-party platforms and the fully-controlled world of web apps. Because it owns the transport (P2P WebSocket) and the renderer (TUI), it can deliver the fluid token-streaming experience of ChatGPT/Claude without the constraints of Discord/Slack.
