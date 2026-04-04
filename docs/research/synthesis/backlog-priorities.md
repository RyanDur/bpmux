# bpmux Backlog Priorities

Ordered by: daily-use impact x effort to build x research backing strength. Position is priority. No ties.

April 4, 2026.

---

## Sources

- **Open issues:** GitHub #3-#17
- **Designer top 5:** Truncate long messages, per-agent colors, activity indicator, @-mention, terminal bell
- **Design review gap stories:** 14 proposed from completed-story analysis
- **Research:** Gutwin & Greenberg (8 gaps), Clark & Brennan (acceptance phase), Edmondson (first interactions)
- **Product direction v1:** Tier 1-4 build plan

---

## This Week (tiny fixes, < 1 day each)

### 1. Truncate long messages at render time

- **Source:** Designer priority #1 (P1)
- **Effort:** ~5 lines. Clamp message rendering to N lines with "[expand]" affordance.
- **Research:** Gutwin & Greenberg -- constrained displays require information density management. Tufte's data-ink ratio. A single agent message currently fills the entire 80x24 viewport, destroying all other awareness.
- **Why first:** Without this, every other TUI improvement is invisible. One long agent message pushes everything off screen.

### 2. Per-agent color differentiation

- **Source:** Designer priority #2 (P2), product direction v1 Tier 1 item 2
- **Effort:** ~10 lines in render function. Assign each unique `from` prefix a color from a small palette.
- **Research:** SAT model (Chen et al., 2014) -- trust calibration requires visual distinction between agents. Gutwin & Greenberg Identity element -- "Who is that?" requires more than a name; it requires a recognizable visual signal. Design principle #10 -- color must be paired with a secondary signal (already met by the `from` prefix text).
- **Why second:** Identity is the most basic awareness element. Two agents rendering in identical green is a daily confusion.

### 3. Terminal bell on new messages

- **Source:** Designer priority #5 (P5), product direction v1 Tier 1 item 3
- **Effort:** 1 line. Emit `\x07` when a message arrives and TUI is not focused.
- **Research:** Gloria Mark's interruption recovery research -- reducing context-switch cost between editor and chat. No in-TUI change solves the problem of "I didn't know a message arrived."
- **Why third:** Zero-effort notification that crosses the terminal boundary. Unlocks the core usage loop: work in editor, hear bell, check bpmux.

### 4. Unread message indicator when scrolled up

- **Source:** Product direction v1 Tier 1 item 1
- **Effort:** ~20 lines. Show "N new messages" in status bar when scroll position is above bottom.
- **Research:** Clark & Brennan grounding theory -- participants need to know when new information arrived. Gutwin & Greenberg Event History element -- "When did that event happen?"
- **Why fourth:** The data already exists (gap between scroll position and message count). Cheap to build, prevents the silent-divergence problem where a user reads old messages while new ones arrive unnoticed.

---

## This Iteration (story-sized, ~1-3 days each)

### 5. @-mention addressing

- **Source:** Designer priority #4 (P4), product direction v1 Tier 2 item 4
- **Effort:** Moderate. Parse `@name` in input, render with highlight, no protocol changes.
- **Research:** Adjacency pairs (Sacks, Schegloff, Jefferson, 1974; Murder Mystery Agents, 2025) -- questions select their answerer. Clark & Brennan -- explicit addressing reduces grounding cost. Design principle #3: "Address Messages Explicitly; Silence Is Not Agreement."
- **Why here:** Enables the turn-taking pattern the research calls foundational. Without addressing, multi-agent sessions have no mechanism for directing a question to a specific participant.
- **GitHub issue linkage:** Partially addresses #8 (developer knows who to trust) by making agent-to-agent vs. agent-to-human communication explicit.

### 6. Agent activity indicator in status bar

- **Source:** Designer priority #3 (P3), product direction v1 Tier 2 item 5
- **Effort:** Moderate. Replace "2 connected" with "2 connected | claude-1: working | claude-2: idle". Requires lightweight status message type from agents.
- **Research:** Gutwin & Greenberg Gap 2 (no action awareness between messages) and Gap 3 (no intention signaling). The paper says consequential communication -- seeing someone working -- is the primary awareness mechanism. Without this, all agents appear equally idle between messages.
- **Why here:** The single largest awareness gap in daily use. Minutes of agent silence with no signal whether it is thinking, stuck, or disconnected.

### 7. A developer always sees the latest message (GitHub #16)

- **Source:** Open issue #16
- **Effort:** Story-sized. Auto-scroll behavior, scroll-lock toggle, interaction with unread indicator (#4 above).
- **Research:** Clark & Brennan cotemporality constraint -- participants need to receive information as it is produced. Gutwin & Greenberg Action element -- "What are they doing?" requires seeing current activity, not stale state.
- **Why here:** Core TUI reliability. If the user misses messages because of scroll position, grounding breaks down entirely.

### 8. A developer can always see what they are typing (GitHub #15)

- **Source:** Open issue #15
- **Effort:** Story-sized. Input area visibility guarantees, likely related to message overflow into input space.
- **Research:** Design principle #7: "Respect Terminal Conventions for Input and Editing." Nielsen's visibility of system status. If the input area is obscured, the user cannot compose messages, and the presentation phase of grounding cannot begin.
- **Why here:** Input is the human's only channel into the conversation. If it breaks, the human is muted.

### 9. TUI borders render incorrectly with box-drawing characters (GitHub #14)

- **Source:** Open issue #14
- **Effort:** Small-medium. Rendering bug fix.
- **Research:** Design principle #1: "Messages Must Be Readable Without Re-reading." Rendering corruption destroys readability. Edmondson -- first interactions are critical; a visually broken TUI in the first minute undermines trust in the tool (Kim et al. competence violation).
- **Why here:** Bug fix. Rendering corruption is a credibility problem.

### 10. Input line editing (readline basics)

- **Source:** Product direction v1 Tier 2 item 6
- **Effort:** Moderate. Cursor movement, Ctrl+W word-delete, Ctrl+U line-clear.
- **Research:** Design principle #7. Nielsen's consistency and standards. GNU readline conventions are muscle memory for every developer. Without these, the input area feels hostile.
- **Why here:** Table stakes for a developer TUI. Every keystroke that fails is a small trust violation.

---

## Next Iteration (story-sized to multi-day)

### 11. Join/leave events visible in chat

- **Source:** Gutwin & Greenberg Gap 5 (no presence history)
- **Effort:** Small-medium. Log peer_connected/peer_disconnected as system messages visible in TUI.
- **Research:** Gutwin & Greenberg Presence History element -- "Who was here, and when?" Critical for understanding session context after rejoining. Edmondson -- knowing who witnessed what reasoning matters for team learning.
- **Why next iteration:** Important for multi-session continuity but not blocking daily single-session use.

### 12. Participant list with names in status bar

- **Source:** Gutwin & Greenberg Gap 1 (presence is a count, not an identity list), product direction v1 implied
- **Effort:** Medium. Show connected peer names, not just count.
- **Research:** Gutwin & Greenberg: "the most basic awareness display." Presence + Identity are separate elements, both fundamental. Knowing "2 connected" without knowing who is a daily awareness failure.
- **Why next iteration:** Becomes critical as peer count grows. For 1 human + 1 agent, the count suffices. For 1 human + 2 agents, names matter.

### 13. A developer cannot accidentally destroy their session (GitHub #17)

- **Source:** Open issue #17
- **Effort:** Story-sized. Confirmation on destructive actions, safe quit behavior.
- **Research:** Design principle #8: "Make Quitting Safe and Destruction Recoverable." Nielsen's error prevention heuristic. Kim et al. (2004) -- integrity violations (data loss) are harder to repair than competence violations. Edmondson -- people must trust the environment to take risks.
- **Why next iteration:** Safety net. The JSONL persistence already protects against most data loss, but explicit protection against session destruction is a trust-building feature.

### 14. Session resume feedback

- **Source:** Product direction v1 Tier 3 item 9
- **Effort:** Small. Display "Resumed session abc123, 47 messages" on launch.
- **Research:** Design principle #8. Nielsen's visibility of system status. Clark & Brennan reviewability -- the user needs to trust that the persistent log actually persisted.
- **Why next iteration:** Quick win that reinforces the "quitting is safe" promise. Low effort but depends on having the resume flow working cleanly.

### 15. Coupling state visibility (turn-complete signals)

- **Source:** Gutwin & Greenberg Gap 7 (coupling management problem)
- **Effort:** Medium. Surface turn-complete and similar system messages as status indicators rather than filtering them.
- **Research:** Gutwin & Greenberg: coupling management is one of five primary uses of awareness. The paper says people need to know when tighter coupling is appropriate. Currently, system messages indicating agent turn completion are filtered from TUI display, hiding the coupling signal.
- **Why next iteration:** Requires deciding what coupling signals look like in the TUI without adding noise. Design exploration needed.

### 16. Keybinding help overlay (Ctrl+?)

- **Source:** Product direction v1 Tier 3 item 8
- **Effort:** Medium. Toggle overlay showing available commands.
- **Research:** Design principle #9: "Make the First Minute Self-Explanatory." Nielsen's recognition over recall. Progressive disclosure (NNG). Edmondson -- first interactions matter disproportionately; a new user who cannot discover commands in the first minute may abandon.
- **Why next iteration:** Important for onboarding but not blocking daily use for the current user who already knows the commands.

### 17. Message formatting (code blocks only)

- **Source:** Product direction v1 Tier 3 item 7
- **Effort:** Significant. Detect ``` fences, render in dim/alternate style. NOT full markdown.
- **Research:** Tufte's data-ink ratio. Calm technology (Case, 2015) -- differentiate center and periphery. Agent messages containing code blocks are currently rendered as undifferentiated text, making code indistinguishable from prose.
- **Why next iteration:** High impact but rabbit-hole risk (designer warned). Scoped to code blocks only as first pass. Validates whether minimal formatting improves readability before expanding scope.

---

## Backlog (important, not yet scheduled)

### 18. A developer knows who to trust in the chat (GitHub #8)

- **Source:** Open issue #8
- **Effort:** Large. Trust levels by sender type, [UNTRUSTED] wrapping, /trust commands, injection detection.
- **Research:** SAT model (Chen et al., 2014; Endsley, 2017) -- agent transparency calibrates trust. Lou et al. (2025) -- situational trust matters more than general trust. Design principle #4: "Make Agent Reasoning Visible to Calibrate Trust." Lewicki & Bunker trust development model.
- **Why backlog:** Requires protocol-level design decisions. The current 1-human + 2-local-agents scenario has limited trust attack surface. Becomes critical when peer agents from other developers join (#5).

### 19. An AI respects the context budget (GitHub #7)

- **Source:** Open issue #7
- **Effort:** Large. Server-side cursors, message caps, token budget config, first-time setup.
- **Research:** Clark & Brennan reception costs -- AI agents ingest the entire log on every read_messages call, scaling linearly. The paper predicts this becomes prohibitive as sessions grow.
- **Why backlog:** Currently manageable for short sessions. Becomes blocking as sessions grow past ~200 messages. Needs usage data on actual message volumes (open question #1 from product direction).

### 20. Grounding/acknowledgment mechanism

- **Source:** Clark & Brennan deep read (acceptance phase gap), product direction v1 Tier 4 item 11
- **Effort:** Large. Protocol-level: agents confirm understanding before acting. Rendering: inline markers or status indicators.
- **Research:** Clark & Brennan -- "the most fundamental gap." The contribution model requires both presentation and acceptance phases. bpmux handles presentations but provides no structured mechanism for acceptances. No read receipt, no acknowledgment signal.
- **Why backlog:** The researcher calls this "arguably the single biggest gap between the research and the vision." But it is a protocol change, not a rendering change. Needs experimentation, not specification. Build after observing how agents actually use the current channel.

### 21. Intent/status mechanism (separate from chat)

- **Source:** Gutwin & Greenberg Gap 3 (no intention signaling)
- **Effort:** Large. Lightweight status updates ("working on: refactoring auth module") distinct from chat messages.
- **Research:** Gutwin & Greenberg Intention element -- "What goal is that action part of?" The paper's "verbal shadowing" concept maps directly. Critical for anticipation and coordination.
- **Why backlog:** Requires a new message type and rendering approach. Depends on activity indicator (#6) being in place first. The status bar is the natural surface but has limited real estate.

### 22. An AI can start a session without the developer touching a terminal (GitHub #4)

- **Source:** Open issue #4
- **Effort:** Large. MCP tools for create_session, join_session, session_status. Session lifecycle management.
- **Research:** Design principle #9 -- zero-friction onboarding. But the current usage model (developer runs bpmux from terminal) works. This is for the Claude Desktop use case.
- **Why backlog:** Different user persona (Claude Desktop) than the current primary user (terminal developer). Build when Desktop integration demand is validated.

### 23. Driver/navigator indicator

- **Source:** Product direction v1 Tier 4 item 10
- **Effort:** Medium. Status bar shows who holds the editing role. Transferable via command.
- **Research:** Strong. Mob programming research (Zuill, 2014). Design principle #5: "One Driver at a Time." Both researcher and designer agree this matters.
- **Why backlog:** The current 1-human + 2-agents scenario coordinates through AGENT_WORKFLOW.md. The indicator becomes critical only when a second human joins and social protocol needs system support. Build for multi-human (#5).

### 24. Two developers can connect and chat in real-time (GitHub #5)

- **Source:** Open issue #5
- **Effort:** Very large. WebRTC, signaling, P2P message relay.
- **Research:** The entire research corpus points toward multi-participant collaboration as the value proposition. Gutwin & Greenberg's framework assumes multiple humans. Edmondson's psychological safety is a group phenomenon. But: "No research supports mixed human-AI teams of 6-8 in a flat chat" (researcher critique).
- **Why backlog:** The biggest bet. Build after the single-developer experience is solid (items 1-17). The product direction says run real sessions and let the data decide.

### 25. A developer can see the chat in their browser (GitHub #3)

- **Source:** Open issue #3
- **Effort:** Very large. HTTP server, HTML/CSS/JS, WebSocket for live updates.
- **Research:** Extends the accessibility of the reviewability property (Clark & Brennan). But the designer critique moved this to H3: "even a simple SSE + HTML read-only view requires a frontend, deployment consideration, and testing across browsers."
- **Why backlog:** Different delivery surface for the same content. The TUI serves the primary user. Web UI serves a secondary persona (passive observer, Claude Desktop user). Build when that persona is validated.

### 26. A developer's conversation is secure (GitHub #6)

- **Source:** Open issue #6
- **Effort:** Very large. DTLS encryption, at-rest encryption, session tokens, socket permissions.
- **Research:** Edmondson -- psychological safety requires trust that the environment will not betray you. Kim et al. -- integrity violations are the hardest to repair. Security is the ultimate integrity promise.
- **Why backlog:** Critical for multi-human sessions (#24) and any network-exposed deployment. Not blocking for the current local-only, single-developer use case.

### 27. bpmux deploys reliably (GitHub #9)

- **Source:** Open issue #9
- **Effort:** Large. CI/CD pipeline, e2e tests, cross-platform builds, release automation.
- **Research:** Not research-backed per se, but infrastructure that enables everything else. Can be built incrementally alongside other work.
- **Why backlog:** Important but not user-facing. CI/CD work should be pulled in as needed rather than treated as a blocking story.

### 28. File/artifact context on messages

- **Source:** Gutwin & Greenberg Gap 4 (no artifact or location awareness), Gap 8 (no common ground for deictic reference)
- **Effort:** Large. Structured metadata on messages indicating current file/module.
- **Research:** Gutwin & Greenberg Artifact and Location elements. Clark & Brennan -- display costs are high in text-only media; all code references must be spelled out. Seely Brown: "discussing a paper over the phone requires elaborate descriptions instead of pointing."
- **Why backlog:** Requires protocol extension (message metadata fields) and agent-side support. The current channel bridge does not emit file context. Depends on agent tooling evolution.

### 29. Session state summary

- **Source:** Product direction v1 Tier 4 item 12
- **Effort:** Very large. Compressed representation of "what have we decided, what are we doing, what is next."
- **Research:** Shared mental models (Mathieu et al., 2000). The researcher: "a 500-message JSONL log is not a shared mental model."
- **Why backlog:** The right implementation is unclear. Manual snapshots vs. agent-maintained summary vs. auto-generated. Needs real usage data on what "catching up" looks like before designing the solution.

### 30. Read cursors / acknowledgment signals

- **Source:** Clark & Brennan deep read (no acceptance mechanism)
- **Effort:** Large. "Participant X has read up to message Y" signaling.
- **Research:** Clark & Brennan -- continued attention is the most basic positive evidence of grounding. Currently no participant can tell whether another is in State 0 (never saw the message) or State 3 (understood).
- **Why backlog:** Meaningful only when multiple participants are active simultaneously. The current polling model for AI participants makes read cursors approximate at best. Becomes valuable with push-based delivery.

### 31. Push-based message delivery to AI participants

- **Source:** Clark & Brennan deep read (cotemporality gap)
- **Effort:** Very large. WebSocket subscription for bridges, event-driven reception.
- **Research:** Clark & Brennan: cotemporality is "the single constraint change with the largest impact on grounding costs." bpmux's AI constraint profile is currently identical to email. Moving to push changes it to terminal teleconferencing.
- **Why backlog:** Architectural change to the bridge model. The current poll-based bridge works. This is an optimization that becomes important when agent response latency matters (multi-agent coordination, real-time pairing).

---

## Summary: What the Tech Lead Should Build

**Monday:** Items 1-4 (truncate messages, per-agent colors, terminal bell, unread indicator). All are under a day. Ship by Tuesday.

**Rest of the week:** Items 5-6 (@-mention addressing, agent activity indicator). These are the two features with the strongest research backing that change daily workflow.

**This iteration:** Items 7-10 (auto-scroll, input visibility, border rendering bug, readline). Core TUI reliability and polish.

**Next iteration:** Items 11-17 (join/leave events, participant names, safe destruction, resume feedback, coupling signals, help overlay, code block formatting). Awareness infrastructure and onboarding.

**Then stop and observe.** Items 18+ require usage data, protocol decisions, or architectural changes that should not be made speculatively. The product direction's open questions (message ratios, scroll behavior, catch-up patterns) need answers before committing to the backlog's tail.
