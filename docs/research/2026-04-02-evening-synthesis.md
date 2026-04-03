# Evening Synthesis: What 25 Research Agents Found

Date: 2026-04-02
Status: Decision document for the morning

---

## 1. The Problem We Are Actually Solving

Not pairing. Not AI participation. Not chat.

**bpmux prevents the cognitive and intent debt that accumulates when developers work with AI in isolation.**

AI coding tools made every developer faster and every team dumber. The evidence is stark: 91% longer PR review times, 67% collapse in junior hiring, code cloning up 4x, refactoring down 60%. Developers think AI makes them 20% faster while it actually slows them 19% (METR study). The "one engineer per repo" trend is not an anomaly -- it is the logical endpoint of optimizing individual throughput without investing in shared understanding.

The core mechanism: when a developer has a 90-minute conversation with an AI about an architectural decision, the reasoning evaporates when the session ends. The code lands. The rationale is gone. Multiply that by every developer on the team, every day, and you get Storey's cognitive debt -- the team can no longer explain why the system is the way it is.

bpmux is not a chat app. It is collective memory for human-AI teams. The persistent message log captures reasoning as it happens. Anyone who arrives later -- a new team member, a fresh AI session, a reviewer -- can read what was decided and why. The log is the shared environment. Coordination emerges from it, not from an orchestrator.

This framing matters because it changes what we build next. A pairing tool needs two people. A chat app needs features. A shared reasoning environment needs to be easy to write to and easy to search. The MCP server and JSONL log we already have are closer to the right answer than we realized.

---

## 2. What We Learned Tonight

### The design debate (4 documents)

A proposal said: embed the LLM client inside bpmux, call APIs directly, manage context windows, handle provider abstraction. A critique said: that turns a chat tool into an AI framework. An editor found factual errors and missing cost estimates. The reconciliation concluded: **the AI should not live inside bpmux.** bpmux is the room, not the AI.

The reconciliation's strongest argument comes from bpmux's own spec. The v1 design says: general-purpose shared environment, no encoded coordination patterns, the Bitter Lesson applies. Embedding LLM client code violates all three principles. The spec explicitly says v1 is communication infrastructure and v2 is when AI runs inside bpmux.

The agreed path: a WebSocket endpoint on the existing axum server (~80-120 lines of Rust) that any external process -- bot, web UI, test harness -- can connect to as a peer. An AI participant is a separate binary that connects via WebSocket, receives messages push-style, and manages its own LLM calls. bpmux never knows or cares that the peer is an AI.

### The bigger problem (4 documents)

The "beyond pairing" research reframed bpmux from a pairing tool to a shared environment for AI reasoning traces. The campfire model: bring your own tool, leave your traces, anyone who comes later can read them. The MCP server is the core interface, not the TUI.

The isolation research documented the damage: knowledge silos from private AI sessions, the mentorship pipeline collapsing, bus factor approaching 1 everywhere. The collaboration-first research found that no tool in the market addresses the core issue -- when two developers each work with their own AI, they build parallel private understandings and neither sees what the other is thinking. bpmux is the only tool designed to make AI reasoning transparent and shared as the primary mode of collaboration.

The collaboration problems research identified seven unsolved coordination problems that AI made worse: architectural alignment, shared understanding (cognitive debt), design rationale (intent debt), conflicting changes at AI velocity, trust and accountability, coordination at scale, and the social fabric of teams. All seven trace back to the same root: reasoning is private and ephemeral.

### History and nature (3 documents)

Every major coordination leap in civilization follows one pattern: externalize reasoning into a shared medium, persist it, share it, let the medium do the coordination work. Cuneiform, the printing press, the Royal Society's journal, military common operating pictures, open source, the Athenian agora. bpmux is the same pattern applied to AI-assisted development.

Nature coordinates the same way. Mycelium networks route nutrients along chemical gradients. Flocking emerges from local rules without global knowledge. The immune system uses cytokine gradients and memory cells. Social insects use quorum sensing. In every case: modify the shared environment, observe the modifications, respond locally. No central authority.

Crisis coordination (Boyd's OODA loop, mission command, NASA voice loops, Bletchley Park, ICS) adds specific insights: shared orientation beats shared decisions; the artifact is the coordination mechanism; structured overhearing enables ambient awareness; tempo has a carrying capacity. Bletchley Park is the closest historical analogue to bpmux -- hut system as bounded contexts, cribs as shared artifacts, traffic analysis as metadata intelligence, stigmergic coordination through work products.

### AI participation (4 documents)

The inspirations research found the IRC bot model is the right architectural pattern: same protocol as humans, push not poll, the bot decides when to speak. The actor model (tokio task per participant with broadcast channel) is the right Rust implementation. The Koala study found 72% preferred reactive AI (respond when mentioned), but users accepted proactive AI when they had controls (threshold slider, mode toggle, rate limit). The Inner Thoughts framework (CHI 2025) provides the decision model: continuously form thoughts, score motivation, speak only above threshold.

The turn-taking research synthesized linguistics (Sacks-Schegloff-Jefferson), game AI (behavior trees), and pair programming (navigator role) into a concrete model: allocated turns (always respond when addressed), open turns (self-select cautiously after growing silence), others' turns (stay silent unless something critical). The AI should be the participant who speaks least, not most. The 5-second rule from pair programming applies: wait before interjecting, the humans might handle it.

The AI pair value research found the clearest signal: **developers want AI that participates in reasoning, not AI that replaces it.** 75% prefer asking a human when accuracy matters -- not because AI is bad, but because you can discuss with humans. The persistent conversation model bpmux proposes is better aligned with what developers report wanting than the tool model. The highest-value AI behaviors: ask questions more than give answers, surface design concerns proactively, make reasoning visible, offer perspective not judgment, maintain context across the conversation.

The cheap experiments document laid out the validation path: Wizard of Oz first (zero code, test the interaction model), shell script bridge second (a few hours, test with real AI responses), then MCP notifications.

### Technical (4 documents)

The external peer model research confirmed: WebSocket on the existing axum server is the right protocol. axum 0.8 has built-in WebSocket support, no new dependencies. The broadcast channel changes from `()` to `Message`. Implementation: ~80-120 lines of new Rust. The bot is a separate process that connects, receives push messages, and sends responses. Multiple bots trivially supported -- each is just another WebSocket connection.

The WebSocket protocol spec is fully designed: identify/welcome/history handshake, message/send/typing/peer_joined/peer_left message types, rate limiting, server-assigned identity (prevents impersonation). The protocol mirrors patterns from Discord, Slack, and Matrix. It coexists with MCP on the same port.

The context management research found: Anthropic's server-side compaction API is the recommended approach (one field in the request), prompt caching yields 90% savings on static prefixes, and effective context is far less than advertised windows (performance degrades well before 1M tokens). Target 50-100K tokens active context. The JetBrains NeurIPS 2025 paper found simple observation masking matches LLM summarization at lower cost.

The tool execution research provides patterns for file sandboxing (path_jail crate), async command execution (tokio::process with kill_on_drop), code search (shell out to rg), and TUI permission dialogs. But the reconciliation concluded: bpmux should NOT build tool execution. That belongs in the companion bot, not in the chat tool.

---

## 3. What We Should Build Next

**Add the WebSocket endpoint to bpmux.** This is the single highest-leverage change. It turns bpmux from a tool that AIs poll into an environment that AIs can join as real-time participants.

Concretely:

1. Change the broadcast channel from `broadcast::channel::<()>` to `broadcast::channel::<Message>` (one-line type change in lib.rs, small refactor of TUI wake logic)
2. Add a WebSocket handler at `/ws` on the existing axum server (~100-150 lines)
3. Implement the identify/welcome/history/message protocol from the spec

This is roughly 150-200 lines of new Rust code with no new dependencies. It enables:
- AI bots that connect as real-time peers (push, not poll)
- A future Web UI connecting via the same protocol
- Test harnesses that participate in sessions programmatically
- Multiple simultaneous external participants

After that, build a proof-of-concept bot as a separate process. It can be 50 lines of Python using `websockets` + `anthropic`. The bot connects, receives messages, responds when mentioned. This proves the architecture end-to-end without touching bpmux's core.

**Honest tradeoffs:**
- Two-process experience (bpmux + bot) is less polished than a single binary with built-in AI. Mitigate with `--with-bot` convenience flag.
- The bot manages its own context window, not bpmux. This is duplicated work if multiple bot implementations exist. Mitigate with a shared library later.
- bpmux cannot enforce AI behavior (system prompt, rate limiting at the AI level). Mitigate with protocol-level rate limiting.
- The external bot will initially be less polished than Claude Code's built-in chat. That is fine -- we are validating the interaction model, not shipping a product.

---

## 4. What We Should NOT Build

**Do not embed an LLM client inside bpmux.** The proposal was well-researched but scoped for v2, not v1. It violates bpmux's own principles (general-purpose, Bitter Lesson, no encoded patterns). The provider abstraction will become load-bearing infrastructure. The system prompt becomes a product decision baked into the binary. The two-path problem (built-in AI vs. MCP clients) will confuse users. Every API change from Anthropic or OpenAI becomes bpmux's maintenance burden.

**Do not build provider abstraction.** Not in bpmux, not even in a companion tool yet. YAGNI. Validate that AI-as-participant is valuable with a single provider (Anthropic) before building abstractions for providers nobody has asked for.

**Do not build tool execution in bpmux.** bpmux is the communication layer. The AI participant that connects via WebSocket can bring its own tools. A coding partner who cannot see code is limited, yes -- but that limitation belongs to the bot, not to bpmux. The existing Claude Code MCP path already gives the AI full coding capabilities. Do not recreate that inside the chat tool.

**Do not build sophisticated trigger policies or turn-taking logic yet.** The research on behavior trees, motivation scoring, and silence detection is excellent and should inform v2. For now, start with the simplest thing: the bot responds when mentioned by name. Tune from there with real usage data.

**Do not prioritize P2P networking over the local experience.** The shared environment works fine as a local log + local MCP server + local WebSocket before it needs to be networked. Solo use and single-machine multi-tool use are the immediate value. WebRTC can wait.

---

## 5. The Cheap Experiment

**Tomorrow morning, run the Wizard of Oz test.**

Cost: zero lines of code. Duration: one working session.

1. Open bpmux in terminal 1 as `ryan`
2. Open bpmux in terminal 2 as `claude` (connect via MCP or just write to the log)
3. Work on a real task. The person in terminal 2 reads the conversation, pastes relevant messages into Claude Code with the prompt: "You are observing a pair programming conversation. Based on this exchange, what would you say as a participant?" Copy the response back into bpmux.
4. Do this for one working session.
5. After the session, write down: Was the AI's input useful? Did you want more or less AI commentary? What kind of input was valuable? Was the chat format awkward for code?

This tests the hardest hypothesis first: **is AI-as-conversation-participant actually valuable for programming?** If two humans find it awkward to pair through a JSONL-backed chat with an AI responding, no amount of infrastructure will fix that.

If the Wizard of Oz shows promise, build the shell script bridge (the one in the cheap-experiments doc -- tail the JSONL, detect @ai, call curl, append the response). This costs a few hours and proves the technical path with real AI responses and real latency.

If both experiments show promise, build the WebSocket endpoint and the Python proof-of-concept bot. That is the real validation.

---

## 6. Open Questions

**Is AI-as-participant actually valuable for programming?** We have strong theoretical arguments (Storey's debt model, stigmergy, the navigator role) and survey data (75% prefer human discussion for accuracy). We have zero empirical evidence from our own usage. The Wizard of Oz test answers this.

**What kind of AI input is valuable in a group chat?** Questions? Design observations? Bug warnings? Code review? All of the above? We will not know until we try. The research suggests questions and strategic observations over tactical code suggestions, but this is borrowed from pair programming literature, not validated for chat.

**Is the chat format adequate for code?** The current message model is plain text. Real programming discussions involve code snippets, file paths, error messages, diffs. Does the TUI need markdown rendering, syntax highlighting, or collapsible sections? The Wizard of Oz test will reveal this quickly.

**What is the right silence threshold for programming?** The Inner Thoughts framework used 10 seconds. Programming involves longer natural silences. The right number might be 30 seconds or more. Only dogfooding will tell.

**Will multiple AI bots in the same session create chaos?** The turn-taking literature has no guidance on multiple AIs following self-selection rules simultaneously. Collision avoidance (random delays, yield-on-conflict) is speculative.

**How expensive is the conversational model?** Back-of-envelope: 50 messages at ~100 tokens each, plus system prompt, AI response ~500 tokens. Per-response cost (Opus, uncached): ~$0.04. A 30-minute session with 50 AI responses: ~$2. With Sonnet: cheaper. With prompt caching: much cheaper. But these are estimates, not measurements.

**Should bpmux ship a companion bot?** The reconciliation says the bot is a separate binary. But should it live in the same repo as a workspace member? Ship in the same installer? Be maintained by the same team? This is a distribution question, not an architecture question, and it can wait until the bot exists.

**What does "search_messages" need to look like?** The campfire model says the highest-leverage feature after the WebSocket is semantic search over the log -- making it easy for latecomers (human or AI) to find relevant prior reasoning. The v1 spec already plans this. Priority should be high.

---

## The Bottom Line

bpmux is already closer to the right thing than we thought. The JSONL log and MCP server are the primitives of a shared reasoning environment. The research validated the core thesis from every angle -- historical, biological, crisis coordination, developer surveys, AI interaction design. The problem is real and getting worse.

The temptation is to build the AI participant inside bpmux. Resist it. Build the WebSocket endpoint. Build a proof-of-concept bot as a separate process. Run the Wizard of Oz test tomorrow. Let the data tell you what to build next.

The campfire does not need to be smart. It needs to be warm, persistent, and easy to find.
