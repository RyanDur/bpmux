# AI as Conversation Participant: Inspirations and Analogies

*Researched April 2, 2026. For the MCP-to-participant pivot.*

The core question: MCP is request/response -- the AI can only speak when asked. We need the AI to receive messages as they arrive and respond naturally, like a participant in a room. What existing systems solve this problem?

---

## 1. IRC Bots -- The Original "AI in a Chat Room"

**How they work:** An IRC bot connects as a regular client over TCP/IP, joins channels, and receives every message in the channel's stream. It is architecturally identical to a human participant -- same protocol, same message flow. The bot maintains state about channels, users, and modes. It responds to predefined events: triggers, mentions, patterns, or timers.

**Key insight:** The bot is not polling. It sits on a persistent connection and messages arrive push-style. The bot's event loop processes each incoming message and decides whether to act. This is the simplest possible architecture for "AI in a chat room" -- and it works because IRC treats bots and humans identically at the protocol level.

**What bpmux should take:** Protocol symmetry. The AI participant should connect to the message stream exactly like a human participant does. No special API, no sidecar. Same pipe, same format. The decision of "should I respond?" is the bot's logic, not the protocol's concern.

**What to avoid:** IRC bots are purely reactive to triggers. They have no model of conversation flow, no concept of "is this a good time to speak?" They respond to pattern matches, not to conversational context. This produces the annoying bot that fires on every keyword.

**Sources:** [IRC Bot - Wikipedia](https://en.wikipedia.org/wiki/IRC_bot) | [IRCHelp Bot FAQ](https://www.irchelp.org/misc/botfaq.html)

---

## 2. Discord/Slack Bots -- WebSocket Event Streams with Intent Filtering

**How they work:** Discord bots connect via a persistent WebSocket to the Gateway, which pushes events (MESSAGE_CREATE, REACTION_ADD, etc.) in real time. The bot sends an Identify handshake, then receives a continuous stream of dispatched events. Gateway Intents let the bot declare which event categories it wants, reducing noise. Slack bots use a similar Events API or Socket Mode.

**Key insight:** The intent/subscription model. The bot does not receive everything -- it declares what it cares about. This is a critical difference from IRC. Discord's architecture separates "what events exist" from "what events you receive," giving the bot control over its input bandwidth.

**What bpmux should take:** The subscription model. The AI participant should be able to declare interest levels: "give me all messages" vs "give me only messages that mention me or my areas of concern." This maps to the proactivity dial -- a reactive AI subscribes to mentions only, a proactive AI subscribes to everything.

**The deeper pattern:** Discord bots handle the heartbeat/keepalive cycle (Opcode 10 Hello, periodic heartbeats). The connection is not fire-and-forget -- it requires active maintenance. bpmux's AI participant similarly needs a lifecycle: connect, maintain presence, handle disconnection gracefully.

**Sources:** [Discord Gateway Documentation](https://docs.discord.com/developers/events/gateway) | [Discord.js Client and Events](https://deepwiki.com/discordjs/guide/2.1-client-and-events)

---

## 3. Game AI -- Behavior Trees and the Perception-Action Cycle

**How they work:** Game NPCs use behavior trees that are "ticked" at a fixed frequency (often 5-10 Hz). Each tick, the tree traverses from root to leaf, evaluating conditions and executing actions. The NPC has a perception system (detection radius, line-of-sight, memory of past events) that feeds into the behavior tree's condition nodes. Actions at the leaves include "speak," "move," "attack," etc.

**Key insight:** The tick-based evaluation cycle. The NPC does not respond to every stimulus instantly. It evaluates its world state at a fixed rate and picks the highest-priority applicable behavior. This prevents the NPC from being overwhelmed by rapid events and gives it a natural "reaction time."

**What bpmux should take:** The perception-evaluation-action cycle. Rather than responding to each message immediately, the AI should accumulate recent messages, evaluate them at intervals (or at natural pauses), and decide on the best action. This is the "batched perception" model -- the AI reads the last N messages as a group, not one at a time.

**The behavior tree analogy for conversation:**
- Root: "Should I participate right now?"
- Branch 1: "Am I directly addressed?" -> Respond
- Branch 2: "Is there a question no one has answered?" -> Offer answer
- Branch 3: "Has the conversation stalled?" -> Prompt or summarize
- Branch 4: "Is someone making a mistake I can help with?" -> Gently intervene
- Fallback: Stay silent

**What to avoid:** Game AI ticks at 5-10 Hz because games run at 30-60 FPS. Chat does not need this frequency. A tick rate of "on each new message, plus a timer for silence detection" is more appropriate.

**Sources:** [Behavior Trees for AI - Game Developer](https://www.gamedeveloper.com/programming/behavior-trees-for-ai-how-they-work) | [Game AI Pro - Behavior Tree Starter Kit](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter06_The_Behavior_Tree_Starter_Kit.pdf)

---

## 4. The Observer Pattern -- Subscribe to the Message Log

**How it works:** In the observer pattern, a subject maintains a list of observers and notifies them of state changes. Observers register interest and receive callbacks. In Rust/Tokio specifically, a `broadcast` channel lets multiple consumers each receive every message sent to the channel.

**Key insight:** This is the simplest correct architecture for bpmux. The message log is the subject. Each participant (human terminal or AI) is an observer. When a message is appended to the log, all observers are notified. The AI observer receives the same notification as the human terminal's display -- protocol symmetry again.

**Tokio implementation path:** A `tokio::sync::broadcast` channel where the session writes messages and both the TUI renderer and the AI participant subscribe. Each gets every message. The AI's subscription handler decides whether to respond.

**What bpmux should take:** This is likely the actual implementation pattern. The message log emits events. The AI subscribes. The AI's response logic is decoupled from the transport.

**What to avoid:** Pure observer pattern has no backpressure. If the AI is slow to process (because it is calling an LLM), messages pile up. The AI needs to handle "I fell behind -- let me catch up" gracefully, reading the batch of missed messages rather than processing each one sequentially.

**Sources:** [Observer Pattern in Rust - DEV Community](https://dev.to/brookzerker/observer-pattern-in-rust-57hl) | [Tokio Channels Tutorial](https://tokio.rs/tokio/tutorial/channels) | [Build a Real Time Chat App with Rust and Tokio](https://www.buildwithrs.dev/blog/build-a-real-time-chat-app-with-rust-tokio)

---

## 5. The Actor Model -- Each Participant Is an Actor

**How it works:** In Erlang/Akka, each actor is an isolated process with its own state and a mailbox. Actors communicate only by sending asynchronous messages. No shared memory. WhatsApp used this model to handle 2+ million concurrent connections per server -- one Erlang process per user connection, with message passing between processes for routing.

**Key insight:** The actor model maps perfectly onto bpmux's participant model. Each participant (human or AI) is an actor. The session/room is an actor that routes messages. When a human types a message, their actor sends it to the session actor, which broadcasts it to all participant actors. The AI actor receives it in its mailbox and decides whether to respond.

**Why this fits bpmux specifically:**
- **Isolation:** The AI actor's state (conversation context, pending LLM call) is private. A slow AI does not block other participants.
- **Mailbox semantics:** Messages queue in the AI's mailbox if it is busy. It processes them in order when ready. This naturally handles the "AI is thinking" case.
- **Supervision:** If the AI actor crashes (LLM timeout, parsing error), a supervisor can restart it without affecting the rest of the session.
- **Location transparency:** The AI actor could run in-process or on a remote machine. The message-passing interface is the same.

**The Rust angle:** Rust does not have Erlang-style actors built in, but `tokio::spawn` + channels approximate it. Each participant is a spawned task with its own channel receiver. The session holds the senders. This is essentially the actor model without the framework.

**Sources:** [Actor Model - dist-prog-book](http://dist-prog-book.com/chapter/3/message-passing.html) | [WhatsApp Erlang Architecture](https://scalewithchintan.com/blog/whatsapp-erlang-architecture-2-billion-users) | [How the Actor Model Meets Modern Systems - Akka](https://doc.akka.io/libraries/akka-core/current/typed/guide/actors-intro.html)

---

## 6. Turn-Taking in Linguistics -- When Should the AI Speak?

**How it works:** Sacks, Schegloff, and Jefferson (1974) identified the foundational model of conversational turn-taking. Key concepts:

- **Turn Construction Units (TCUs):** A turn is built from units (sentences, clauses, phrases) that have recognizable completion points.
- **Transition Relevance Places (TRPs):** The end of a TCU is a point where speaker change becomes relevant. Participants project when a TRP is approaching based on syntax, intonation, and pragmatics.
- **Turn Allocation Rules:** (1) Current speaker selects next speaker (e.g., asks someone a question). (2) If no one is selected, anyone may self-select. (3) If no one self-selects, current speaker may continue.
- **Minimized gaps and overlaps:** The system produces remarkably smooth transitions with minimal silence and minimal simultaneous talk.

**Key insight for bpmux:** The AI should follow these rules, not override them.
- Rule 1: If someone asks the AI a question (selects it), it should respond. This is the "reactive" baseline.
- Rule 2: If a question is posed to the room and no one answers (a TRP passes without self-selection), the AI may self-select. This is the "helpful participant" mode.
- Rule 3: If the AI is already speaking (producing a multi-part response), it may continue.

**The silence signal:** In human conversation, a gap longer than about 700ms at a TRP is noticeably long and signals trouble. In async text chat, the equivalent is "no one has responded in N seconds." The AI can use silence duration as a trigger: short silence = others are typing, long silence = the floor is open.

**The dominance problem:** The biggest risk is the AI violating turn-taking norms by self-selecting too often, producing too-long turns, or not yielding the floor. Research consistently shows that participants who dominate turn-taking are perceived negatively. The AI must be the participant who speaks least, not most.

**Sources:** [Sacks, Schegloff & Jefferson (1974)](http://www.cs.columbia.edu/~julia/cs4706/Sacks_et_al_1974.pdf) | [Turn-taking - Wikipedia](https://en.wikipedia.org/wiki/Turn-taking) | [Social Communication - Turn-Taking](https://socialcommunication.truman.edu/hidden-social-dimensions/turn-taking/)

---

## 7. Pair Programming -- Driver/Navigator as Role Model

**How it works:** In driver/navigator pair programming, the driver writes code while the navigator reviews and thinks strategically. Key communication norms:

- The navigator communicates *intent*, not keystrokes ("we need to handle the error case" not "type if err != nil").
- Good navigators wait for natural pauses before offering suggestions. They do not interrupt mid-thought.
- The navigator parks ideas (on sticky notes) and raises them at appropriate moments rather than blurting them out.
- Roles switch regularly so neither person is passive for too long.

**Key insight for bpmux:** The AI's default role should be navigator, not driver. It observes the work, thinks about the bigger picture, and speaks at natural pauses with strategic observations rather than tactical instructions.

**When the AI should speak (navigator rules):**
- When asked directly
- When it sees a bug or risk the driver has not noticed
- When the current approach is heading toward a dead end
- When a pause suggests the driver is stuck
- When a decision is being made without considering an important tradeoff

**When the AI should stay quiet:**
- When the humans are in flow
- When the humans are discussing something the AI has no useful input on
- When someone just spoke and needs time to process the response
- When the AI would be repeating what someone already said

**The "parking lot" pattern:** The navigator writes down ideas rather than saying them immediately. The AI equivalent: the AI can accumulate observations and deliver them in a batch at an appropriate moment, rather than responding to each message individually.

**Sources:** [Martin Fowler - On Pair Programming](https://martinfowler.com/articles/on-pair-programming.html) | [Thoughtworks - Effective Navigation](https://www.thoughtworks.com/insights/blog/effective-navigation-in-pair-programming) | [Driver/Navigator Role Definitions](https://gist.github.com/jordanpoulton/607a8854673d9f22c696)

---

## 8. The "Room" Metaphor -- Putting It All Together

**The synthesis:** What if we combine all of the above into a coherent model?

The bpmux session is a **room**. Participants -- human and AI -- are in the room together. The room has a **message log** (the shared artifact everyone can see, like a whiteboard in a meeting room). The architectural patterns map as follows:

| Concern | Pattern | Source |
|---------|---------|--------|
| How does the AI connect? | Same protocol as humans (IRC bot symmetry) | IRC bots |
| How does the AI receive messages? | Subscribe to broadcast channel (observer/actor) | Discord, Tokio broadcast |
| How does the AI decide to speak? | Behavior tree with perception cycle | Game AI |
| When does the AI speak? | Turn-taking rules + silence detection | Linguistics |
| What role does the AI play? | Navigator in pair programming | Pair programming |
| How does the AI avoid dominating? | Contribution threshold + user controls | Koala study |

---

## 9. Two Key Research Papers -- Direct Precedent

### Koala: Controlling AI Participation in Group Chat (2025)

Researchers built Koala, a Slack-based agent using Llama, with two modes:

- **Reactive:** Only responds when mentioned by name.
- **Proactive:** Evaluates every message, scores its potential response, and posts if the score exceeds a threshold.

**Critical finding:** 72% of participants preferred the reactive variant. The proactive variant was perceived as distracting and dominating -- 73% of ideas came from Koala in proactive mode, but only 33% were selected as top ideas. Participants wanted controls: a mode toggle, a contribution threshold slider, and the ability to relegate responses to threads.

**Implication for bpmux:** Start reactive. Make proactivity a user-controlled dial, not the default. The threshold model (AI scores its own response and suppresses low-scoring ones) is directly implementable.

**Source:** [Controlling AI Agent Participation in Group Conversations](https://arxiv.org/html/2501.17258v1)

### Inner Thoughts: Proactive Agents with Autonomous Timing (2025)

This framework gives the AI a parallel stream of "inner thoughts" that runs alongside the conversation. The AI:

1. **Triggers** on conversational events (new message, silence > 10 seconds)
2. **Retrieves** relevant memories (weighted by recency, importance, relevance)
3. **Forms thoughts** using both fast (System 1) and deliberate (System 2) processing
4. **Scores motivation** across eight dimensions: relevance, information gap, expected impact, urgency, coherence, originality, balance, dynamics
5. **Speaks** only if motivation exceeds a configurable threshold

**Critical finding:** The framework distinguishes "allocated turns" (someone asked the AI) from "open turns" (anyone can speak). For open turns, the AI self-selects only if its motivation score is high enough. For allocated turns, it always responds. There is also an "interrupt threshold" -- higher than the normal threshold -- for cases where the AI should break into someone else's turn.

**Implication for bpmux:** The eight-dimension motivation scoring is overengineered for a first implementation, but the core idea is right: the AI should continuously form thoughts and speak only when a thought crosses a threshold. The distinction between allocated vs open turns maps directly to the turn-taking rules from linguistics.

**Source:** [Proactive Conversational Agents with Inner Thoughts](https://arxiv.org/html/2501.00383v2)

---

## 10. Stanford Generative Agents -- The Full Autonomy Endpoint

**How it works:** Park et al. (2023) created Smallville, a virtual town of 25 AI agents that live, work, converse, and form relationships autonomously. Each agent has:

- **A memory stream:** Complete record of experiences in natural language
- **A retrieval function:** Surfaces relevant memories using recency, importance, and relevance scoring
- **A reflection function:** Periodically synthesizes recent memories into higher-level insights
- **A planning function:** Generates day plans, decomposes them into hour and minute chunks

Agents spontaneously organized a Valentine's Day party -- spreading invitations, asking each other on dates, and showing up at the right time -- from a single seed notion that one agent wanted to throw a party.

**Key insight:** This is the far end of the autonomy spectrum. The agents are not waiting to be asked. They have goals, make plans, and initiate conversations. This is probably too much autonomy for bpmux's AI participant, but the memory and retrieval architecture is directly relevant.

**What bpmux should take:** The memory retrieval model. When the AI considers responding, it should retrieve relevant context from the conversation history weighted by recency and relevance -- not just the last N messages, but semantically relevant earlier messages too. The reflection function (periodically summarizing) connects to bpmux's existing interest in conversation summaries.

**What to avoid:** Full agent autonomy. The bpmux AI should not have its own agenda or initiate conversations unprompted (at least not in v1). It should be a responsive participant, not an autonomous agent with goals.

**Sources:** [Generative Agents: Interactive Simulacra of Human Behavior](https://arxiv.org/abs/2304.03442) | [Stanford HAI - Computational Agents](https://hai.stanford.edu/news/computational-agents-exhibit-believable-humanlike-behavior)

---

## Synthesis: The Recommended Architecture

Based on all of the above, the architecture that emerges for bpmux's AI participant:

**1. Transport: Actor-per-participant with broadcast.**
Each participant (human or AI) is a Tokio task with a channel receiver. The session broadcasts messages to all participants. Protocol symmetry -- the AI receives messages exactly like the TUI does.

**2. Perception: Batched, not per-message.**
The AI accumulates messages and evaluates them in batches at natural pause points (silence > N seconds, or when directly addressed). This prevents the AI from responding to every message and gives it a natural "reading the room" delay.

**3. Decision: Behavior tree with turn-taking rules.**
- Directly addressed? Always respond.
- Open question, no one answering, silence growing? Self-select.
- Conversation flowing well between humans? Stay quiet.
- Someone making a factual error or heading toward a problem? Flag it.
- Default: silence.

**4. Contribution quality gate.**
Before posting, the AI scores its own response. Below threshold = discard. This is the Koala finding and the Inner Thoughts motivation score, simplified.

**5. Role: Navigator, not driver.**
The AI's default persona is the pair-programming navigator: strategic observations, potential risks, alternative approaches. Not line-by-line tactical input.

**6. User control: Reactivity dial.**
Start in reactive mode (respond only when addressed). Let users increase proactivity. The dial adjusts the silence threshold, the self-selection willingness, and the contribution quality gate.
