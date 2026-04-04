# Conduit Model Reevaluation: Grounding Theory Stress Test

*April 4, 2026. Does the conduit model survive Clark & Brennan?*

---

## The Tension

The conduit model says: bpmux is the message bus, not the interface. The TUI is the logger. Users interact through their existing AI tools.

Clark & Brennan say: communication is not transmission -- it is the joint establishment of mutual belief. Every contribution requires a presentation phase AND an acceptance phase. A message sent without evidence of understanding is not a grounded contribution -- it is an ungrounded utterance.

The question: can a message bus support grounding, or does grounding require bpmux to be something more than a bus?

---

## 1. Is the Conduit Model Compatible with Grounding Theory?

**Short answer: No. A pure message bus cannot support the acceptance phase.**

Here is why. Clark & Brennan's contribution model has two phases:

- **Presentation:** A sends a message. bpmux handles this. The JSONL log records it. WebSocket broadcasts it. Done.
- **Acceptance:** B provides evidence that B understood what A meant. This is where the conduit model breaks.

A pure conduit -- a dumb pipe -- transmits messages. It does not know whether a message was received, read, parsed, or understood. It cannot distinguish between a participant in State 0 (never saw the message) and State 3 (understood it fully). The pipe does not care. That is what makes it a pipe.

But grounding requires caring. The deep read found that bpmux's constraint profile for AI participants is identical to email: reviewability and revisability, nothing else. The paper is explicit that email is one of the worst media for grounding because it provides no mechanism for the acceptance phase. A conduit that replicates email's constraint profile inherits email's grounding failures.

The critical gap is not that bpmux fails to transmit messages -- it transmits them reliably. The gap is that transmission is only half the contribution model. The other half -- evidence of understanding -- has no structural support. It happens only when participants voluntarily provide it, and the deep read established that AI agents do not provide it voluntarily. They skip the acceptance phase unless the medium makes them do it.

**A pure message bus can carry acceptance signals, but it cannot require them.** That is the distinction. TCP carries HTTP, but TCP does not enforce HTTP semantics. Similarly, bpmux can carry acknowledgment messages, but a pure conduit has no opinion about whether acknowledgments happen. Grounding theory says they must happen. The conduit model says it is not the bus's job to enforce that.

This is not a compatibility problem. It is an insufficiency problem. The conduit model is not wrong -- it is incomplete.

---

## 2. Where on the Spectrum Should bpmux Sit?

The spectrum:

```
Dumb pipe ---- Opinionated conduit ---- Communication protocol ---- Application platform
   |                  |                         |                          |
 TCP/UDP         SMTP (envelope           HTTP (methods,             Slack (threads,
                  format, but              status codes,             reactions, read
                  no delivery              content types,            receipts, typing
                  confirmation)            caching rules)            indicators, bots)
```

**bpmux should sit at "opinionated conduit."** Here is the reasoning.

Moving to "communication protocol" (HTTP-level opinions) would mean bpmux enforces request-response patterns, requires specific message types, mandates acknowledgment flows. This contradicts the conduit model's core insight: bpmux should not own the interaction pattern. Developers use Claude Code, Claude Desktop, or whatever tool they already use. bpmux should not force those tools into a protocol they were not designed for.

Moving to "application platform" (Slack-level opinions) would mean bpmux owns the user experience: threading, reactions, presence indicators as first-class UI features. This directly contradicts the conduit model. bpmux is not the interface.

But staying at "dumb pipe" means bpmux cannot prevent the grounding failures that Clark & Brennan predict and the deep read confirmed. The log becomes a broadcast medium. Cognitive debt accumulates silently.

**"Opinionated conduit" means:** bpmux defines message semantics (what kinds of messages exist and what they mean) without dictating interaction patterns (when those messages must be sent or how participants must respond). It is SMTP, not HTTP. SMTP defines the envelope format -- from, to, subject, body -- and the delivery mechanism, but it does not enforce that the recipient reply, or that the reply address the sender's content, or that the conversation reach mutual understanding. SMTP has opinions about structure but not about behavior.

bpmux should have opinions about structure: what a message looks like, what metadata it carries, what signals are available. It should not have opinions about behavior: when to acknowledge, how to respond, whether silence means agreement.

---

## 3. The Minimum Addition That Satisfies Clark & Brennan

Clark & Brennan require evidence of understanding. The minimum addition is: **make it possible to provide that evidence, without requiring it.**

Three specific additions, in order of necessity:

### 3a. Read cursors (State 0 vs. State 1+)

The deep read identified the most fundamental gap: no participant can tell whether another participant has even seen a message. Read cursors -- "participant X has read up to message Y" -- distinguish State 0 (never noticed) from States 1-3 (noticed, with varying levels of understanding).

This is metadata on the transport layer, not a new message type. The bus already knows when a participant calls `read_messages`. Recording and exposing that fact is a conduit-level operation. It does not change what bpmux is. It makes the existing pipe observable.

**What stays the same:** Messages flow through the bus unchanged. No new message types. No required responses. The bus does not force anyone to read.

**What changes:** The bus records what it already knows -- which participants have consumed which messages -- and makes that information available. This is analogous to SMTP delivery receipts: the mail server already knows whether the message was delivered; the receipt just exposes that fact.

### 3b. Message type field (presentation vs. acceptance)

Add an optional `type` field to the message schema. Two values to start: `message` (default, a presentation) and `ack` (an acceptance -- "I have read and understood message Z").

This does not require acknowledgments. It provides a structured way to express them. The difference matters. A conduit that carries typed messages is still a conduit. It is SMTP with a Content-Type header, not HTTP with mandatory response codes.

AI agents can be instructed (via system prompts, hooks, or conventions) to send `ack` messages for consequential content. Humans can choose to acknowledge or not. The bus carries both message types identically. It has no opinion about whether `ack` messages appear.

**What stays the same:** The bus routes messages. The TUI displays them. The log persists them. No enforcement.

**What changes:** The message schema gains a `type` field. The vocabulary of what can be expressed through the bus expands to include acceptance signals. This is the minimum structural support for the acceptance phase.

### 3c. Reply-to reference (grounding context)

Add an optional `in_reply_to` field: a message ID that this message responds to. This allows any message -- whether a presentation, an acknowledgment, or a relevant next turn -- to be explicitly linked to the message it grounds.

The deep read found that understanding costs increase when "the addressee has to imagine appropriate contexts for both the sender and the message, and to remember what the message is in response to." Reply-to references make grounding context explicit rather than requiring inference from sequence position.

**What stays the same:** The bus does not require replies. It does not enforce threading. The log remains a flat, sequential record.

**What changes:** Messages can optionally point to the message they respond to. Participants and tools can use this to reconstruct grounding chains. The bus carries the reference without interpreting it.

### What this does NOT include

- No mandatory acknowledgment flow. The bus does not reject messages that lack acknowledgments.
- No threading. The flat log remains flat. `in_reply_to` is a reference, not a thread.
- No presence indicators. These belong in the transport layer (WebSocket connection state), not the message schema.
- No typing indicators. These are UI concerns for the TUI, not bus concerns.
- No enforcement of any grounding behavior. The bus provides the vocabulary. Participants choose whether to use it.

---

## 4. Does This Change What bpmux IS, or Just What It DOES?

**It changes what bpmux does. It does not change what bpmux is.**

The conduit model defines bpmux's identity: the message bus, not the interface. The three additions above do not touch that identity. bpmux still routes messages. The TUI still observes. Users still interact through their existing tools.

What changes is the expressiveness of the bus. Before: the bus carries undifferentiated messages. After: the bus carries messages with type information and reference links, and it tracks consumption state. The bus becomes a more capable conduit, not a less conduit-like system.

The analogy holds: SMTP did not stop being a mail transport protocol when it added MIME types. It became a more expressive transport. The envelope gained structure. The protocol's identity -- move mail from sender to recipient -- did not change.

There is one important caveat. The thesis v2 says: "bpmux must make agents acknowledge understanding structurally -- through explicit addressing conventions, acceptance signals, and a norm that silence is ambiguous, never confirmation." This goes beyond what the conduit model can deliver. "Making agents acknowledge" is enforcement. A conduit provides the mechanism for acknowledgment but does not compel it.

The enforcement comes from outside the bus: system prompts, hooks, conventions, norms. The conduit provides the vocabulary (`ack` messages, read cursors). The participants' tooling provides the enforcement (Claude Code's system prompt says "acknowledge consequential messages before acting on them"). This division preserves the conduit model: bpmux provides infrastructure, not policy.

**The conduit model survives, but it needs an addendum:** bpmux is the message bus, not the interface. The bus carries typed, referenceable messages and tracks consumption. Grounding behavior is enforced by participants and their tooling, not by the bus.

---

## 5. What Storey's Framework Says

Storey's cognitive debt framework asks: does the team's shared understanding keep pace with the code? Intent debt asks: is the rationale for decisions externalized and accessible?

**A pure conduit is insufficient for debt prevention.** Here is why.

Cognitive debt accumulates when team members cannot articulate why design decisions were made or how components interact. A pure message bus captures what was said. But cognitive debt is not about what was said -- it is about what was understood. A log full of ungrounded messages (presentations without acceptances) is a log full of potential understanding that was never verified. The team has a record of what was broadcast. They do not have a record of what was mutually established.

Intent debt accumulates when rationale evaporates. A pure conduit preserves the messages, which preserves the rationale in the log. This is valuable. But intent debt also accumulates when rationale is misunderstood -- when Developer B reads Developer A's reasoning and draws the wrong conclusion, and nobody catches it because there was no acceptance phase. The rationale exists in the log, but the understanding of it was never grounded.

**The additions in section 3 partially address this.** Read cursors provide evidence that participants have at least consumed the log (reducing State 0 cognitive debt -- "I didn't even know this discussion happened"). Message types provide a structured way to signal understanding (reducing State 2 cognitive debt -- "I read it but misunderstood"). Reply-to references make the grounding chain visible (reducing intent debt -- "this decision was made because of that discussion, and here is evidence the team understood it").

But "partially" is the honest assessment. Storey's framework would say that a conduit -- even an opinionated one -- is a necessary but not sufficient condition for debt prevention. The sufficient condition is that grounding actually happens: participants actually acknowledge, actually confirm understanding, actually repair misunderstandings. The conduit provides the infrastructure. The culture provides the practice. The system prompts provide the nudge.

This is consistent with thesis v2's position that psychological safety is a prerequisite. The bus can carry acceptance signals. Whether participants send them depends on whether they feel safe admitting confusion and asking for clarification. No amount of protocol design substitutes for that.

---

## Summary: What Changes and What Stays

### What stays the same

- **bpmux is the message bus, not the interface.** Users interact through their existing tools. The TUI is the observer.
- **The persistent log is the primitive.** Append-only, ordered, reviewable. This does not change.
- **bpmux does not enforce interaction patterns.** No mandatory acknowledgments, no required response flows, no threading.
- **The conduit model is the right architectural decision.** Grounding theory does not invalidate it. It reveals its boundary.

### What changes

- **The bus becomes more expressive.** Messages gain optional `type` and `in_reply_to` fields. The bus tracks read cursors. The vocabulary of what can be expressed expands.
- **The conduit model gets an addendum.** bpmux provides grounding infrastructure (mechanisms for acceptance signals). Grounding enforcement lives outside the bus, in system prompts, hooks, and team norms.
- **The thesis v2 language tightens.** "bpmux must make agents acknowledge understanding structurally" should be read as: bpmux must provide the structural mechanisms for acknowledgment; agent tooling must use them. The bus enables. The tooling enforces.

### The revised conduit model

bpmux is the message bus, not the interface. The bus carries typed, referenceable messages and tracks consumption state. It provides the structural mechanisms for grounding -- acknowledgment signals, read cursors, reference links -- without requiring any specific grounding behavior. Grounding enforcement is the responsibility of participants and their tooling. The bus is opinionated about what can be expressed, not about what must be expressed.

The TUI remains the logger. Users remain in their existing tools. The log remains the primitive. The bus just got a little smarter about what it carries.
