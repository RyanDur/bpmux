# Design Reconciliation: Where Should the AI Live?

Date: 2026-04-02
Status: Recommendation
Input: `2026-04-02-ai-participant-design.md` (proposal), `2026-04-02-design-critique.md` (critique), editor notes, inspirations, turn-taking research, AI pair value research, v1 spec, why-bpmux

---

## The Core Disagreement

The proposal says: bpmux should own the conversation loop. Put the LLM client inside bpmux, call APIs directly, manage context windows, handle provider abstraction. bpmux becomes the agent loop owner.

The critique says: that is the wrong boundary. bpmux is a communication layer. The AI should connect as an external peer, like an IRC bot. bpmux should be the chat room, not the AI framework.

Both are partially right. Both are partially wrong. Neither is the right v1 answer.

---

## What the Proposal Gets Right

**The problem is real.** MCP's request/response model makes the AI a polled tool, not a participant. The AI cannot respond spontaneously. It cannot follow the conversation. Something needs to change for AI to be a real participant.

**History replay is the correct mental model.** Every AI chat tool works the same way: the AI is stateless, the application replays conversation history on each call. The AI "sees" messages because the application shows them, not because the AI is listening. The proposal correctly identifies this as the universal architecture.

**The trigger policy matters.** The proposal at least raises the question of when the AI should speak. The addressed-by-default, configurable-proactivity model is reasonable, even if the critique correctly notes it reduces to command invocation.

**Multi-provider will eventually matter.** Provider lock-in is a real risk for a tool-agnostic system. The proposal is right that bpmux should not be Claude-only forever.

## What the Proposal Gets Wrong

**It violates bpmux's own principles.** The v1 spec says bpmux is a "general-purpose shared environment" that follows the Bitter Lesson -- general methods outperform domain-specific knowledge encoding. It explicitly says "no encoded coordination patterns" and "how teams coordinate is their choice, not bpmux's." But the proposal encodes a specific coordination pattern: bpmux decides when the AI speaks, what context it sees, what system prompt it uses, and what provider it talks to. That is not a general-purpose environment. That is an opinionated AI orchestration layer embedded in a chat tool.

**It makes bpmux responsible for LLM client infrastructure.** The critique is right that the provider abstraction will become load-bearing. Context window management, token counting, streaming SSE parsing, retry logic, rate limiting, prompt caching, provider-specific quirks -- this is the hard part of building an LLM application. The proposal estimates 500-800 lines; the editor notes calculate 1000-1400 lines with tests. Either way, bpmux takes on a maintenance burden that is tangential to its core purpose: communication.

**It creates the two-path problem.** MCP clients and a built-in AI participant will not coexist cleanly. Users will not understand the distinction. Two AIs in the same session -- one managed by bpmux, one external via MCP -- will produce confused, conflicting behavior without coordination that the proposal does not design.

**It competes with the user's existing tools on their home turf.** A developer using Claude Code already has an AI that sees code, edits files, runs commands, manages context, and streams responses. The proposal's built-in AI participant cannot see code, cannot edit files, cannot run commands. It is strictly less capable than what the user already has. The only thing it adds is "following the conversation" -- but that is a feature that could be added to the MCP path instead.

## What the Critique Gets Right

**The IRC bot model preserves protocol symmetry.** The inspirations research's top recommendation is that the AI should connect to the message stream exactly like a human participant -- same protocol, same message flow. The critique's external-peer model achieves this. The proposal's built-in model does not: the AI participant is a special internal component, not a peer on the same protocol.

**bpmux should stay a communication layer.** The v1 spec is clear: bpmux is a message bus with chat interfaces. The P2P infrastructure, message protocol, and multi-participant chat are the foundation. The critique correctly identifies that embedding an LLM client pushes bpmux away from this identity.

**Cheap experiments should come before production architecture.** The critique's suggestion of a Wizard of Oz test (paste AI responses manually), a pipe script (50 lines of bash that tails the log and calls curl), or improving the existing MCP path are all cheaper ways to test the core hypothesis. The proposal jumps from research to production architecture without validating that AI-as-participant is valuable in practice.

## What the Critique Gets Wrong

**It dismisses the real problem too casually.** Saying "use a Unix socket or local WebSocket" for the external peer model is hand-waving. The AI is stateless. An external process that connects to bpmux's message stream still needs to: maintain conversation context, decide when to respond, call an LLM API, and stream the response back. All the same complexity exists -- it just lives in a separate binary. Moving it outside bpmux does not make it disappear; it makes it someone else's problem. And "someone else" is likely bpmux's authors anyway, since no such external AI participant process exists today.

**The IRC bot model is not quite right either.** IRC bots work because IRC is a standardized protocol with a rich ecosystem of client libraries. bpmux's message format is proprietary. An external AI participant connecting to bpmux would need a bpmux-specific client library, which bpmux would need to provide and maintain. The protocol symmetry is real at the conceptual level but requires work at the implementation level.

**It underestimates the value of the hypothesis.** The critique asks whether AI-as-participant is valuable and suggests testing before building. Fair. But the entire purpose of bpmux is this hypothesis. The why document says bpmux exists to prevent cognitive and intent debt by making AI reasoning visible. If the AI cannot participate in the conversation spontaneously, the reasoning stays invisible. Testing this hypothesis is not premature -- it is the point.

---

## What the Spec's Own Principles Say

Reading the v1 spec carefully, it answers the question more clearly than either the proposal or the critique realize.

**1. General-purpose environment, no encoded patterns.** The spec says coordination emerges from the shared space, not from a manager directing it. A built-in AI participant with trigger policies, system prompts, and provider management is a manager directing coordination. This favors the critique's external-peer model.

**2. Stigmergy.** The spec says bpmux enables stigmergic coordination -- the trace left by one participant's action stimulates the next action by another. This does not require bpmux to own the AI's decision loop. It requires bpmux to deliver messages reliably so that any participant (human or AI, internal or external) can observe and respond. This favors the communication-layer model.

**3. The Bitter Lesson.** General methods outperform domain-specific encoding. Building a specific AI orchestration layer inside bpmux is domain-specific encoding. Building a good message transport that any AI process can connect to is a general method. This clearly favors the critique.

**4. v1 is communication, v2 is the coding environment.** The spec explicitly says v1 builds the P2P infrastructure, message protocol, and multi-participant chat. v2 wraps it in an integrated coding experience where "Claude and other models run inside bpmux." The proposal is trying to do a piece of v2 during v1. The spec's own roadmap says: foundation first.

**5. bpmux never depends on the AI being present.** The spec says bpmux is "a chat that AIs can participate in, not a system that requires AIs to function." Embedding the AI participant inside bpmux blurs this line. An external peer that connects and disconnects cleanly preserves it.

The spec's principles consistently point toward the critique's position: bpmux is the room, not the AI.

---

## What the User Research Says

The AI pair value research tells a clear story:

- Developers want AI that participates in reasoning, not AI that replaces it.
- The #1 frustration is context blindness -- AI that does not understand the codebase or conversation history.
- 75% of developers prefer asking a human when accuracy matters, because you can discuss with humans.
- The value proposition is making AI reasoning visible and discussable.

None of this requires the AI to live inside bpmux. It requires the AI to be able to read the conversation and contribute to it. Whether the AI lives inside bpmux or connects as an external peer is an implementation detail from the user's perspective. What matters is the experience: the AI follows the conversation and speaks at appropriate moments.

The turn-taking research adds nuance: the AI should speak least, not most. It should follow turn-taking norms. It should self-filter before posting. These are behaviors of the AI participant, wherever it lives. They are not properties of the transport layer.

---

## The Third Option: Make the Existing Path Good

Neither the proposal nor the critique fully reckons with the fact that bpmux already has an AI participation mechanism: MCP. The proposal acknowledges it exists but dismisses it because MCP is request/response. The critique mentions improving MCP but does not develop the idea.

What if the right answer is not "build a new thing" (proposal) or "build a different new thing" (external peer process), but "make the existing thing work better"?

The MCP path's real limitation is not that it is request/response. It is that the AI tool (Claude Code, etc.) has to decide when to poll. If bpmux could notify the AI tool that new messages have arrived, the AI tool could decide whether and when to read and respond. This preserves the existing architecture while solving the spontaneity problem.

**Concrete approach: push notifications over the existing MCP connection.**

MCP supports server-initiated notifications. bpmux already has a broadcast channel that wakes consumers when new messages arrive. If the MCP server sends a notification to connected clients when new messages arrive, the AI tool can subscribe to those notifications and decide whether to act. The decision of when to respond stays with the AI tool -- where it belongs, because the AI tool has the full context of what the developer is doing, not just the chat.

This is not hypothetical engineering. It is connecting two things that already exist: bpmux's broadcast channel and MCP's notification mechanism.

**What this gets right:**

- bpmux stays a communication layer. No LLM client code, no provider abstraction, no context management.
- The AI tool (Claude Code, etc.) keeps its full capabilities: code access, file editing, command execution. The built-in participant proposed cannot do any of these.
- No two-path problem. There is one path: MCP. It just works better.
- The AI tool decides when to respond based on its full context (conversation + codebase + user activity), not just the chat.
- Protocol symmetry is maintained at the MCP level: every AI client gets the same notifications.
- It follows the Bitter Lesson: improve the general transport, do not encode specific AI behavior.

**What this does not solve:**

- It does not work for users who do not have an AI coding tool. But those users are not bpmux's target audience.
- The AI tool must implement the "follow the conversation" behavior. Claude Code does not do this today. But this is Claude Code's concern, not bpmux's.
- It is less impressive as a demo. "bpmux has a built-in AI that responds in the chat" is a better pitch than "bpmux sends notifications to your existing AI tool." But demos are not architecture.

---

## But What About Users Who Want a Simpler Experience?

The proposal has one real strength the critique does not address: some users will want to start a bpmux session and have an AI just be there, without running Claude Code or any other tool separately. The "built-in assistant" use case is real.

The honest answer: this is a v2 concern. The v1 spec says so explicitly. v1 is communication infrastructure. v2 is when AI runs inside bpmux. Trying to deliver a piece of the v2 experience during v1 creates the architectural problems both the critique and the editor notes identify.

If the v1 MCP notification path proves that AI-as-participant is valuable, then v2 can build the integrated experience with the benefit of real usage data about what context the AI needs, how often it should speak, and what provider flexibility matters. Building the built-in AI now means making all those decisions with zero data.

---

## The Reconciled Recommendation

**For v1: Invest in the MCP notification path. Do not build a built-in AI participant.**

1. Add MCP server-initiated notifications when new messages arrive. Connected AI tools receive a push signal instead of polling.
2. Include enough metadata in the notification for the AI tool to decide whether to read: message count, sender identity, whether the AI was mentioned.
3. Document the "AI as conversation participant" pattern so that AI tool authors (including the bpmux team, if building a companion tool) know how to implement it.
4. Run the cheap experiments the critique suggests: Wizard of Oz sessions, bash pipe scripts, manual Claude Code participation. Gather data on whether AI-as-participant is valuable before building infrastructure.

**For v1.x or v2: Build the external AI participant as a companion binary.**

If the experiments validate the hypothesis:

1. Build `bpmux-ai` (or similar) as a separate binary that connects to bpmux via MCP, receives notifications, and manages the AI conversation loop.
2. This binary owns: provider abstraction, context management, trigger policy, system prompts.
3. bpmux owns: message transport, notifications, authentication, the room.
4. The two can be distributed together (like `git` and `git-daemon`) but are architecturally separate. This preserves the communication-layer boundary.
5. Users who want the simple experience run `bpmux --with-ai` which starts both processes. Users who want to bring their own tool just run `bpmux`.

This approach:
- Respects the spec's own principles (general-purpose, Bitter Lesson, v1 = communication)
- Avoids the proposal's architectural overreach (no LLM client in the chat tool)
- Avoids the critique's hand-waving (concrete path forward, not just "someone else builds it")
- Tests the hypothesis cheaply before committing to infrastructure
- Preserves the option to build the integrated experience in v2 with real data

---

## What Each Side Should Take from This

**To the proposal authors:** The research is thorough and the architecture is sound in isolation. The problem is not competence but scope. bpmux is not the right place for LLM client code in v1. The provider abstraction, context management, and trigger policy are all real concerns -- they should live in a companion tool that connects to bpmux, not inside bpmux itself. The inspirations research's richer architecture (behavior trees, batched perception, contribution quality gates) deserves a dedicated home, not a simplified shim inside a chat transport.

**To the critique authors:** The "IRC bot as external peer" model is the right conceptual direction, but it needs to be grounded in what exists today. MCP is the connection mechanism. Push notifications are the missing piece. The critique correctly identifies the problems but the alternative it proposes ("Unix socket or local WebSocket") ignores the protocol bpmux already has. The answer is not a new transport; it is a better MCP.

---

## The Minimum Viable Test

Before building anything:

1. **Week 1: Wizard of Oz.** Two developers pair using bpmux as it exists. One developer (or a third observer) reads the bpmux log, pastes relevant messages into Claude Code with the prompt "You are observing a pair programming conversation. Based on this exchange, what would you say as a participant?" Copy the response back into bpmux. Do this for 2-3 sessions. Record: Was the AI's input useful? Did the developers want more or less AI commentary? What kind of input was valuable?

2. **Week 2: The bash pipe.** A shell script that tails the JSONL log, detects `@ai` messages, assembles recent context, calls the Anthropic API with curl, and appends the response to the log file. The TUI already renders log entries. This proves the interaction pattern before investing in production code.

3. **Week 3: MCP notification prototype.** Add a simple notification to the MCP server when new messages arrive. Test whether Claude Code (or a simple MCP client script) can use the notification to participate more naturally than pure polling.

Each experiment costs days, not weeks. Each answers a specific question. Together they validate (or invalidate) the core hypothesis before any production architecture is committed.

---

## Decision Summary

| Question | Recommendation | Rationale |
|---|---|---|
| Should the AI live inside bpmux? | No (v1). Possibly as companion binary (v2). | bpmux is a communication layer. The spec says so. |
| What should v1 build? | MCP push notifications | Improves the existing path without adding LLM client complexity |
| Should we build provider abstraction? | Not in bpmux. In a companion tool, if validated. | YAGNI until the hypothesis is proven |
| How do we test the hypothesis? | Wizard of Oz, bash pipe, MCP notification prototype | Cheap experiments before production architecture |
| Is the critique right that the proposal is wrong? | Mostly, but the critique's alternative is underspecified | The real answer is "improve MCP," not "build a different external thing" |
| Is the proposal right that direct API calls are the architecture? | Yes, for the companion tool. No, for bpmux itself. | The architecture is sound; the location is wrong |
| What about users who want a simple built-in AI? | v2 concern, after v1 data validates the model | Do not optimize the experience before proving the value |
