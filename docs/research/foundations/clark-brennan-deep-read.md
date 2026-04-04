# Clark & Brennan (1991) "Grounding in Communication" -- Deep Read for bpmux

*April 4, 2026. A close reading of the foundational paper on grounding theory, mapped to bpmux's architecture and design space.*

**Source:** Clark, H. H., and Brennan, S. A. (1991). Grounding in Communication. In L.B. Resnick, J.M. Levine, & S.D. Teasley (Eds.), *Perspectives on socially shared cognition*. Washington: APA Books. Chapter 7, pp. 127-149.

**Why this paper matters for bpmux:** Clark & Brennan formalized how people establish mutual understanding -- the process they call *grounding*. bpmux exists to prevent cognitive and intent debt in human-AI teams. Grounding theory tells us exactly what "mutual understanding" requires, what makes it expensive, and what happens when it fails. If bpmux is infrastructure for shared understanding, this paper is the specification for what shared understanding actually is.

---

## 1. The Full Grounding Model

### What grounding is

Grounding is the collective process by which participants in a communication establish that what has been said has been understood. It is not a property of individual speakers or listeners -- it is a joint activity requiring coordinated effort from both sides.

The core definition from the paper: "The contributor and his or her partners mutually believe that the partners have understood what the contributor meant to a criterion sufficient for current purposes." This is the **grounding criterion**.

Three things to note about this definition:

1. **Mutual belief, not individual belief.** It is not enough that B understands A. Both A and B must believe that B understands A. This is a higher bar than mere transmission.
2. **Sufficient for current purposes.** The criterion is not absolute comprehension. It shifts with what the participants are trying to accomplish. Getting a phone number right requires verbatim accuracy. Getting the gist of a story requires much less.
3. **It is a process, not a state.** Common ground must be updated moment by moment. It is not established once and then assumed.

### How grounding works: the contribution model

Every contribution to a conversation has two phases:

**Presentation phase:** A presents an utterance for B to consider. A assumes that if B gives sufficient evidence of understanding, A can believe B understood.

**Acceptance phase:** B gives evidence that she believes she understands what A meant. Once A registers that evidence, both believe B has understood.

It takes both phases for a contribution to be complete. A presentation without acceptance is not a contribution -- it is an ungrounded utterance.

### The four states of understanding

After a presentation, the receiver may be in one of four states:

- **State 0:** B did not notice that A uttered anything.
- **State 1:** B noticed A uttered something but could not parse it.
- **State 2:** B correctly heard the utterance but did not understand its meaning.
- **State 3:** B understood what A meant.

The grounding process exists to move participants from whatever state they are in to State 3, or at least to make the current state mutually known so it can be repaired.

### Three forms of positive evidence

People do not merely look for the absence of misunderstanding. They actively seek positive evidence that understanding has occurred:

1. **Acknowledgments.** Back-channel responses: "uh huh," "yeah," "m," head nods. These signal that the partner is passing up the opportunity to initiate a repair -- and by implication, believes they understand so far.
2. **Relevant next turn.** The strongest form of evidence. If A asks a question and B provides an appropriate answer, the answer itself is evidence that B understood the question. If the answer is inappropriate (as in the Chico Marx examples the paper uses throughout), it is evidence of misunderstanding.
3. **Continued attention.** The most basic form. As long as the partner continues to attend in the expected way -- looking, not turning away, not looking puzzled -- the speaker has reason to believe understanding is proceeding.

The paper explicitly notes that requiring positive evidence does not create an infinite regress, because relevant next turns and continued attention do not require separate presentations. The acceptance process can spin out for multiple turns but usually terminates when the partner initiates a relevant next contribution.

### Mapping to bpmux

**What bpmux currently provides:**

- The JSONL message log is the presentation medium. When an AI sends a message via `send_message`, it has completed a presentation phase.
- The `read_messages` tool allows a participant to receive presentations. An AI that reads the log and then sends a relevant response has completed the strongest form of acceptance: the relevant next turn.
- The persistent log means presentations do not evaporate. Unlike speech (which the paper calls "evanescent"), bpmux messages persist indefinitely.

**What bpmux currently lacks:**

- **No acceptance phase for most messages.** When Human A types a message in the TUI, there is no mechanism for confirming that Claude B has read and understood it. The message enters the log, but there is no evidence it has been grounded. This is the same gap that exists in email -- the paper identifies it explicitly.
- **No acknowledgment mechanism.** There is no equivalent of "uh huh" or a read receipt. An AI that reads the log gives no signal to other participants that it has done so. The `read_messages` call is invisible to the group.
- **No state visibility.** No participant can tell whether another participant is in State 0 (never saw the message), State 1 (saw but could not parse), State 2 (parsed but did not understand), or State 3 (understood). The log records what was said but not what was received.

**The grounding criterion and "sufficient for current purposes":**

This is directly relevant to bpmux's design. The paper says the criterion shifts with purpose. In a pair programming session, "I'm going to refactor the auth module" needs to be grounded to a lower criterion than "The API key is `sk-abc123`." bpmux currently treats all messages identically -- there is no way to mark a message as requiring verbatim accuracy versus gist-level understanding. Whether it should is a design question worth considering.

---

## 2. The Eight Constraints

Clark & Brennan identify eight properties that vary across communication media and shape how grounding works. Here is each constraint, its definition, and its status in bpmux.

### 1. Copresence

**Definition:** A and B share the same physical environment. They can see and hear what each other is doing and looking at.

**bpmux:** Absent. Participants are in separate terminal sessions, potentially on different machines. There is no shared physical environment. However, all participants share the same JSONL log and can observe the same message history -- this is a form of *virtual* copresence over the message log itself, though not over the participants' activities.

### 2. Visibility

**Definition:** A and B are visible to each other. They can see facial expressions, gestures, posture.

**bpmux:** Absent. Terminal-based. No visual channel whatsoever. AI participants do not have faces. Human participants cannot see each other.

### 3. Audibility

**Definition:** A and B communicate by speaking. They can hear intonation, timing, emphasis.

**bpmux:** Absent. All communication is text. No audio channel. No intonation, no emphasis beyond what can be expressed in prose. Markdown formatting is available but not rendered in the TUI.

### 4. Cotemporality

**Definition:** B receives at roughly the same time as A produces. Production and reception are synchronous.

**bpmux:** Partially present. The WebSocket protocol broadcasts messages to connected peers in near real-time. The TUI updates when new messages arrive via the broadcast channel. But AI participants only receive messages when they call `read_messages` -- they poll rather than listen. A Claude Code instance might not read the log for minutes or hours after a message is sent. For human TUI users, cotemporality is high. For AI participants, it depends entirely on how frequently they check.

**This is one of bpmux's most significant grounding constraints.** The paper says cotemporality is what separates conversation-like media from letter-like media. bpmux is conversation-like for humans but letter-like for AIs.

### 5. Simultaneity

**Definition:** A and B can send and receive at the same time. Both directions of the channel are open at once.

**bpmux:** Partially present. Multiple participants can send messages concurrently -- the session log handles concurrent appends. But there is no mechanism for simultaneous back-channel communication. When one participant is "speaking" (composing a message), others cannot signal acknowledgment mid-utterance. The closest analog would be if an AI could observe a human typing in real-time, which it cannot.

### 6. Sequentiality

**Definition:** A's and B's turns cannot get out of sequence. Messages are received in the order they are sent, without interleaving from unrelated conversations.

**bpmux:** Present. The JSONL log is strictly ordered by timestamp. All participants see the same sequence. This is one of bpmux's structural advantages over email, where (as the paper notes) "a message and its reply may be separated by any number of irrelevant messages or activities." In bpmux, the log is a single sequential conversation.

### 7. Reviewability

**Definition:** B can review A's messages after the fact. The utterance persists as an artifact.

**bpmux:** Strongly present. This is bpmux's signature property. The JSONL log is permanent, append-only, and readable by any participant at any time. Every message ever sent is reviewable. The paper identifies reviewability as the defining advantage of written media over spoken media. bpmux has this to a degree that exceeds even email, because the log is a single ordered artifact rather than scattered across inboxes.

### 8. Revisability

**Definition:** A can revise a message privately before sending it.

**bpmux:** Present for AI participants, partially present for humans. An AI composing a `send_message` call can formulate and reformulate before sending. A human typing in the TUI can edit before pressing Enter. Once sent, messages cannot be revised -- the log is append-only.

### Summary table

| Constraint | Face-to-face | Telephone | Email | bpmux (human) | bpmux (AI) |
|---|---|---|---|---|---|
| Copresence | Yes | No | No | No | No |
| Visibility | Yes | No | No | No | No |
| Audibility | Yes | Yes | No | No | No |
| Cotemporality | Yes | Yes | No | Yes (via WS) | No (polling) |
| Simultaneity | Yes | Yes | No | Partial | No |
| Sequentiality | Yes | Yes | No | Yes | Yes |
| Reviewability | No | No | Yes | Yes | Yes |
| Revisability | No | No | Yes | Yes | Yes |

**bpmux's constraint profile for AI participants is identical to email.** The paper characterizes email as having only reviewability and revisability. That is exactly what bpmux provides to AI participants who poll via `read_messages`. For human TUI users, bpmux adds cotemporality and sequentiality, making it closer to "terminal teleconferencing" in the paper's taxonomy.

This is a significant finding. If bpmux wants to be a grounding medium for AI participants, it needs to move its AI constraint profile closer to terminal teleconferencing (adding cotemporality) rather than remaining at the email level.

---

## 3. Grounding Techniques

The paper catalogs specific techniques people use to establish mutual understanding. Each is worth examining for AI applicability.

### Techniques for grounding references

1. **Alternative descriptions.** A refers to "that young gentleman from the park"; B confirms by offering "Joe Wright you mean?" This technique tests whether both parties have identified the same referent. In programming terms: if one participant says "the auth module" and another responds by naming specific files or functions, the reference is being grounded.

   *bpmux application:* AI participants naturally do this when they reference code. An AI might say "I updated the session handler" and another might respond "the `handle_ws` function in `transport/ws.rs`?" This is grounding, and bpmux's persistent log makes these exchanges reviewable. But there is no mechanism to *prompt* this behavior -- it depends entirely on the AI's own conversational instincts.

2. **Indicative gestures.** Pointing, looking at, touching an object. Impossible in text-based media. In a programming context, this would be sharing a screen and pointing at a line of code.

   *bpmux application:* Absent. bpmux has no mechanism for pointing at code. The closest substitute is quoting specific code in a message, which is expensive in tokens but achievable.

3. **Referential installments.** Establishing the identity of a referent before saying something about it. "Take the spout -- the little one that looks like the end of an oil can [pause for confirmation] -- and put that on the opening." The speaker secures the reference before proceeding.

   *bpmux application:* This technique requires cotemporality -- the speaker pauses for confirmation before continuing. Since AI participants lack cotemporality in bpmux, they cannot use installments. They must either pack everything into one message (risking misidentification) or send multiple messages and wait for responses (expensive in turns and time). This is a real cost of the email-like constraint profile.

4. **Trial references.** Presenting an uncertain reference with a marker asking for confirmation. "A man called Annegra?" with rising intonation.

   *bpmux application:* AI participants could adopt this technique in text. "I believe you're referring to the WebSocket heartbeat logic (in `transport/ws.rs`, around the `HEARTBEAT_INTERVAL` constant)?" The question mark is the try marker. This is possible in bpmux today and is probably underused.

### Techniques for grounding verbatim content

1. **Verbatim displays.** Repeating back what was heard: "The number is 12345." "12345." "That's right." The receiver proves they got it by repeating it.

   *bpmux application:* Relevant for API keys, configuration values, specific commands. An AI could confirm: "Setting the MCP port to 4117 as specified." This is available today but not prompted by any convention.

2. **Installments.** Breaking verbatim content into chunks and confirming each. Used for phone numbers, addresses, recipes.

   *bpmux application:* Possible but impractical given the lack of cotemporality for AI participants. An AI would have to send a message, wait for confirmation, send the next chunk, wait again. The overhead is prohibitive.

3. **Spelling.** Explicitly spelling out critical words.

   *bpmux application:* Directly relevant when discussing identifiers, variable names, module paths. The text medium actually helps here -- you can write `HEARTBEAT_INTERVAL` exactly, without risk of mishearing.

### What the paper implies for AI agents

The paper's grounding techniques evolved for humans who share a rich, cotemporal, often copresent environment. AI agents in bpmux operate in a much more constrained medium. The implication is that AI agents need to compensate by:

- **Over-specifying references.** Since there are no gestures or installments available, every reference to code, concepts, or decisions should be explicit enough to be unambiguous on first reading.
- **Using trial references proactively.** Adding try markers ("I believe you mean X?") to check understanding before acting on assumptions.
- **Employing verbatim displays for critical content.** Repeating back specific values, file paths, and commands to confirm shared understanding.
- **Treating their own messages as presentations that require acceptance.** An AI that sends a message and immediately acts on it without waiting for confirmation is skipping the acceptance phase. Sometimes this is appropriate (when the grounding criterion is low), but for consequential decisions it is not.

---

## 4. Costs of Grounding

The paper identifies eleven costs that vary across media. Each shapes how participants choose to ground.

### 1. Formulation costs

The time and effort to plan an utterance. It costs more to plan complicated utterances, retrieve uncommon words, or describe unfamiliar objects. It costs more to formulate perfect utterances than imperfect ones.

*bpmux:* AI participants have low formulation costs for text but high formulation costs for *precise* text about complex code. The cost is measured in tokens and latency, not effort. Humans have high formulation costs when writing technical messages.

### 2. Production costs

The act of producing the utterance. Speaking is cheap, typing is more expensive, handwriting most expensive.

*bpmux:* Low for AI (text generation is fast). Moderate for humans (terminal typing). The asymmetry matters -- an AI can produce a 500-word message in seconds; a human might take minutes. This creates an imbalance in turn-taking dynamics.

### 3. Reception costs

Listening is generally easy; reading is harder. It also costs to wait while the speaker produces a turn.

*bpmux:* Reception cost is a significant concern. AI agents receive the *entire* log on every `read_messages` call. As the session grows, reception costs scale linearly with log length. For humans, the TUI shows the recent history, which limits reception cost. For AIs, there is currently no filtering, pagination, or summarization of the log -- they ingest everything.

### 4. Understanding costs

The cost of comprehending what was said. Compounded when contextual clues are missing. The paper says email is harder to understand because "the addressee has to imagine appropriate contexts for both the sender and the message, and to remember what the message is in response to."

*bpmux:* This is a direct hit. bpmux messages lack threading. A message that says "I disagree" requires the reader to scan the log to figure out what is being disagreed with. The sequential log helps (it provides temporal context), but as conversations grow long and interleave topics, understanding costs rise. The paper's observation about email applies directly: without cotemporality, "the addressee has to imagine appropriate contexts."

### 5. Start-up costs

The cost of initiating a new discourse. Minimal face-to-face. Higher for phone (dial, wait, identify). Higher still for email (access software, find address, compose, no delivery guarantee).

*bpmux:* Start-up costs are moderate for humans (launch bpmux, join session). For AI participants, start-up costs are tied to the bridge and MCP connection -- currently handled by Claude Code's hook system, which auto-connects. Once connected, start-up cost per contribution is low. But the initial context loading (reading the full log to understand the session state) is a substantial start-up cost that the paper does not fully anticipate because it predates AI participants with large context windows.

### 6. Delay costs

The cost of delaying an utterance to plan it more carefully. In cotemporal media, delays are expensive because they are interpreted -- a long pause might mean the speaker is dropping out, or disagreeing, or confused. In non-cotemporal media (email, letters), delays are costless.

*bpmux:* For human TUI users, delays are noticeable but less costly than face-to-face because there are no misinterpretable pauses. For AI participants, delays are invisible -- there is no expectation of immediate response, and long gaps between `read_messages` calls go unnoticed. This means AI participants can take all the time they need for formulation, which trades delay costs for lower formulation costs. The paper explicitly notes this trade-off: "In writing letters, we can take our time planning and revising each sentence."

### 7. Asynchrony costs

The cost of imprecise timing. In conversation, people time utterances with great precision -- starting exactly at the completion of a prior turn, timing acknowledgments to mark what they acknowledge. Without cotemporality, timing is impossible.

*bpmux:* High for AI participants. An AI cannot time its response to land immediately after a question. It might respond minutes later, by which time other messages may have intervened. For human TUI users, asynchrony costs are lower because of real-time WebSocket delivery, but still higher than face-to-face.

### 8. Speaker change costs

The cost of one speaker stopping and another starting. Low face-to-face (frequent, well-marked transition points). High in keyboard teleconferencing. Higher still in email and letters, where "changing the speaker is a little like starting up a communication from scratch."

*bpmux:* The paper notes that "one effect of high speaker change costs is that people try to do more within a turn." This is exactly what we observe with AI agents -- they tend to write long, comprehensive messages rather than short conversational ones, because each turn is expensive. This is a rational adaptation to the medium's cost structure, but it makes grounding harder because the recipient must ground a large block of content all at once rather than incrementally.

### 9. Display costs

The cost of pointing, gesturing, showing objects. Easy face-to-face; impossible in text-only media.

*bpmux:* High. All reference must be done through verbal description. Code snippets, file paths, and error messages must be spelled out in text. No screen sharing, no pointing.

### 10. Fault costs

The cost of making a mistake. Some faults lead to failures in understanding; others make the speaker look foolish. In conversation, faults are expected because speech is spontaneous. In email, "faults are not as easily justified, because the sender has already had a chance to revise them."

*bpmux:* AI messages in bpmux are revisable before sending, so faults carry higher social (or credibility) cost than in conversation. An AI that sends a message with incorrect information to the shared log has made a permanent, reviewable mistake. The append-only log means faults cannot be quietly corrected -- they must be publicly repaired with a follow-up message. This raises the stakes for accuracy.

### 11. Repair costs

The cost of fixing a mistake. Self-repairs are cheapest. Other-initiated repairs are more expensive. In non-cotemporal media, "repairs initiated or made by others become very costly indeed."

*bpmux:* Repair costs are high. If an AI sends an incorrect message, correcting it requires a new message that references the error. The original incorrect message remains in the log. If another participant has already read and acted on the incorrect message, the repair may require unwinding multiple actions. The paper's observation applies: "speakers will try hard to avoid relying on others to repair misunderstandings" in non-cotemporal media. AI participants should invest more in formulation to avoid faults, since repairs are expensive.

### Where bpmux sits on the cost spectrum

For AI participants, bpmux has a cost profile similar to email: low delay costs, high repair costs, high speaker change costs, high asynchrony costs, high display costs. The rational strategy the paper predicts for this profile is: invest heavily in formulation (compose careful, complete messages), minimize faults (because repairs are expensive), and do more per turn (because speaker changes are expensive).

For human TUI users, bpmux is closer to terminal teleconferencing: lower delay costs than email, moderate speaker change costs, sequential, cotemporal via WebSocket.

---

## 5. Least Collaborative Effort

### The principle

The paper replaces the traditional "principle of least effort" (which instructs the speaker to minimize their own effort) with a stronger principle:

> **The principle of least collaborative effort:** In conversation, the participants try to minimize their collaborative effort -- the work that both do from the initiation of each contribution to its mutual acceptance.

This is not about minimizing the speaker's effort. It is about minimizing the total effort of all participants combined. Sometimes this means the speaker does more work (formulating a clearer message) to save the listener from doing more work (asking for clarification). Sometimes it means the speaker does less work (issuing an imperfect utterance) because the listener can cheaply fill in the gaps.

### Key consequences

1. **Self-repair is preferred over other-repair.** Speakers prefer to fix their own mistakes because self-repair costs fewer total turns. An interlocutor repairing requires extra turns and the speaker must accept the repair anyway.

2. **Self-initiated repair is preferred over other-initiated repair.** The speaker catching their own error costs fewer total turns than the listener prompting a correction.

3. **Improper utterances are sometimes cheaper.** When it costs more to formulate a perfect utterance than to issue an imperfect one and have the partner help, speakers rationally choose imperfection. Try markers, installments, and invitations to complete are all techniques for distributing the work.

### Application to human-AI teams in bpmux

The principle of least collaborative effort has direct implications for how AI agents should communicate in bpmux:

**AI agents should front-load effort.** In bpmux's email-like medium for AI participants, other-repair is very expensive (requires a full round-trip through the log). Self-repair before sending is comparatively cheap (the AI can revise before calling `send_message`). The rational strategy is for AI agents to invest more in formulation -- checking their own messages for clarity, completeness, and correctness before sending -- because the cost of a repair cycle in bpmux is high.

**But not *too much* effort.** The principle says minimize *total* effort, not *speaker* effort. An AI that writes a 2000-word message to be maximally clear may impose higher reception and understanding costs on the human reader than a shorter, less perfect message that invites a quick clarifying question. The optimal effort level depends on the grounding criterion -- how precisely does this particular content need to be understood?

**Turn-taking dynamics are asymmetric.** Human-to-AI grounding has different cost structures than AI-to-AI grounding. A human typing a clarifying question is expensive (slow production). An AI sending a clarifying message is cheap (fast production). This means the least collaborative effort strategy differs depending on who is speaking and who is listening:

- When an AI speaks to a human: invest in formulation, keep messages focused, make references explicit, because the human's cost of requesting repair is high (slow typing).
- When a human speaks to an AI: less formulation investment is rational, because the AI's cost of requesting clarification is low (fast production, cheap processing).
- When an AI speaks to an AI: formulation costs and repair costs are both low, so the optimal strategy converges toward quick, iterative exchanges -- but only if cotemporality is available. Without it (bpmux's current state for AI-to-AI communication), each round-trip is expensive in wall-clock time, so front-loading effort is still preferred.

**The persistent log changes the collaborative effort calculus.** In ephemeral conversation, a failed grounding attempt wastes the effort of both participants and produces nothing. In bpmux, a failed grounding attempt is still recorded. A future participant reading the log can learn from the failure -- they see what was misunderstood and how it was repaired. This means the "waste" of grounding failures is partially recovered as documentation. The paper does not consider this because it predates persistent, shared communication logs with reviewability by third parties.

---

## 6. What the Paper Says About Text-Based Communication

### Keyboard teleconferencing

The paper discusses "keyboard teleconferencing" (what we would now call a chat room or instant messaging) as a distinct medium. Its characteristics:

- Cotemporality (messages arrive in real-time)
- Sequentiality (ordered conversation)
- Reviewability (recent messages visible on screen)
- No copresence, visibility, audibility, simultaneity

The paper notes that in keyboard teleconferencing, "it is difficult to time an acknowledgment precisely, and trying to do so may interrupt the other typist. So the cost of an acknowledgment is higher in this medium." People compensate by using fewer acknowledgments and doing more per turn.

*bpmux (human TUI) maps closely to this category.* The TUI provides cotemporality via WebSocket, sequentiality via the ordered log, and reviewability via the persistent log. The prediction follows: human users of bpmux will use fewer acknowledgments and write longer messages than they would face-to-face.

### Email

The paper characterizes email as having only reviewability and revisability. No cotemporality, no sequentiality, no simultaneity.

Key observations:

- "Email is neither cotemporal nor sequential. That makes understanding harder because the addressee has to imagine appropriate contexts for both the sender and the message, and to remember what the message is in response to."
- Start-up costs are high (access software, find address, start message, no delivery guarantee).
- Delay costs are nil (delays are not even noticeable).
- Speaker change costs are high (changing speakers "is a little like starting up a communication from scratch").
- Repair costs are very high ("repairs initiated or made by others become very costly indeed").
- Fault costs are high because "the sender has already had a chance to revise them."

*bpmux (AI participants) maps to this category almost exactly.* The one exception is sequentiality -- bpmux provides it (single ordered log) while email does not (messages scattered across inboxes). This is a meaningful advantage. The paper identifies lack of sequentiality as a major understanding cost for email; bpmux avoids that cost.

### Personal letters

Even more constrained than email -- the paper's most limited medium. Only reviewability and revisability, same as email, but with higher production costs (handwriting vs. typing) and higher start-up costs. Not directly relevant to bpmux except as a lower bound.

### The general pattern for text media

The paper's framework predicts that text-based, non-cotemporal media will produce:

1. Longer, more carefully composed messages (trading delay costs, which are low, for formulation costs, which reduce fault costs).
2. Fewer acknowledgments (because timing them is impossible or expensive).
3. More content per turn (because speaker change costs are high).
4. Higher understanding costs (because contextual clues from copresence, visibility, and audibility are absent).
5. Higher repair costs (because other-initiated repair requires full round-trips).
6. Greater reliance on the reviewability of the record (because content cannot be re-heard, but it can be re-read).

All six predictions describe what we actually observe in bpmux sessions. AI agents write long messages, do not acknowledge, pack multiple topics per message, occasionally misunderstand context, are expensive to correct, and benefit from being able to re-read the log.

---

## 7. Synthesis: What bpmux Gets Right and What It Is Missing

### What bpmux gets right from a grounding theory perspective

**Reviewability is bpmux's structural advantage.** The paper identifies reviewability as the property that distinguishes written media from spoken media. bpmux takes this further than any medium the paper considers: the log is permanent, complete, ordered, and accessible to all participants including those who join later. Face-to-face conversation has no reviewability. Even email has only partial reviewability (scattered across inboxes, subject to deletion). bpmux's append-only JSONL log is a maximally reviewable record.

**Sequentiality is preserved.** Unlike email, bpmux maintains a single ordered conversation. The paper identifies lack of sequentiality as a major source of understanding costs in email. bpmux avoids this.

**The `from` field enables attribution.** The paper discusses grounding as a process between identified participants. bpmux's message format (`from: "ryan:claude-code"`) provides clear attribution, which is necessary for tracking who has said what and who needs to ground with whom.

**Revisability before sending.** Both human and AI participants can compose and revise before sending. The paper predicts this will lead to higher-quality presentations with fewer faults, which is desirable.

### What bpmux is missing from a grounding theory perspective

**No acceptance mechanism.** This is the most fundamental gap. The paper's contribution model requires both a presentation phase and an acceptance phase. bpmux handles presentations (sending messages) but provides no structured mechanism for acceptances. There is no read receipt, no acknowledgment signal, no way to know whether a message has been received, let alone understood. Participants must infer grounding from relevant next turns -- which works when they come, but provides no signal when they do not.

**No cotemporality for AI participants.** AI participants poll the log via `read_messages`. They have no push notification, no real-time feed, no way to receive a message at the time it is sent. The paper is clear that cotemporality is what separates conversation-like media (which support cheap grounding) from letter-like media (which make grounding expensive). Moving AI participants from polling to event-driven reception (e.g., WebSocket subscription for the bridge) would fundamentally change the grounding cost structure.

**No threading or topic marking.** The paper discusses how understanding costs increase when the addressee must "imagine appropriate contexts for both the sender and the message, and to remember what the message is in response to." A flat sequential log becomes harder to ground against as it grows. Threading, quoting, or reply-to mechanisms would reduce understanding costs by making the context of each message explicit.

**No presence or attention signals.** The paper identifies "continued attention" as the most basic form of positive evidence. bpmux has a peer count in the TUI status bar (counting MCP and WebSocket connections), but no mechanism for signaling that a specific participant is currently reading or attending to the conversation. Typing indicators, online/offline status, or "last read" markers would provide the attention evidence that the paper identifies as fundamental.

**No mechanism for grounding references to code.** The paper describes specialized techniques for grounding references (alternative descriptions, indicative gestures, installments, trial references). bpmux has no affordance for referencing code -- no ability to link to files, highlight lines, or embed code context. All code references must be spelled out in message text, which the paper would predict increases both formulation and understanding costs.

### Design implications ranked by grounding theory impact

1. **Push-based message delivery to AI participants.** Moving from polling to push (WebSocket subscription for bridges) would add cotemporality for AI participants, the single constraint change with the largest impact on grounding costs according to the paper's framework.

2. **Read cursors or acknowledgment signals.** Even a simple "participant X has read up to message Y" would provide the basic acceptance evidence that the paper says is required for grounding. It would not prove understanding, but it would distinguish State 0 (never saw it) from at least State 1 (noticed it).

3. **Reply-to or quoting mechanism.** Allowing messages to reference specific prior messages would reduce understanding costs by making context explicit rather than requiring inference from sequence position.

4. **Presence indicators.** Signaling which participants are currently connected and active would provide the "continued attention" evidence that the paper identifies as the most basic form of positive evidence.

5. **Code reference affordances.** Supporting structured references to files, line ranges, or commits would reduce the display costs the paper identifies as high in text-only media.

---

## 8. The Deepest Insight for bpmux

The paper's deepest insight is not about any single constraint or technique. It is this: **communication is not transmission. Communication is the joint establishment of mutual belief.** A message sent is not a message grounded. A message received is not a message understood. Grounding is the process of moving from transmission to mutual understanding, and it requires effort from all participants.

bpmux currently handles transmission well. The JSONL log, the WebSocket broadcast, the MCP bridge -- these ensure messages are transmitted and persist. But the paper tells us that transmission is only the presentation phase. The acceptance phase -- the evidence of understanding, the repair of misunderstandings, the negotiation of shared meaning -- is what makes communication actually work. That phase is currently left entirely to the participants' own initiative.

For human-human pairs, this is fine. Humans naturally seek and provide grounding evidence. But AI agents do not have the same instincts. They will read the log, process it, and act on it without signaling comprehension. They will not ask clarifying questions unless prompted. They will not acknowledge receipt unless instructed. They will assume understanding rather than verifying it. In Clark & Brennan's terms, they will skip the acceptance phase.

If bpmux is infrastructure for shared understanding in human-AI teams, the grounding theory says: the infrastructure is not done until the acceptance phase is as well-supported as the presentation phase. The persistent log captures what was said. What is missing is a record of what was understood.
