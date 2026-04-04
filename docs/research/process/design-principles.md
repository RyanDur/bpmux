# Design Principles for the bpmux TUI

Authority document. All future TUI design decisions reference these principles and the research backing them.

---

## 1. Messages Must Be Readable Without Re-reading

**Principle**: Every message in the chat log must establish its sender, intent, and audience within the first line.

**Research backing**: Clark & Brennan's "Grounding in Communication" (1991) identifies eight constraints that affect grounding cost. Text chat is strong on reviewability and sequentiality but completely lacks copresence, visibility, and audibility. Without nonverbal cues and backchannel signals ("uh-huh," nodding), participants cannot confirm understanding implicitly. The grounding cost is higher per message than in face-to-face or voice communication.

**Application to bpmux**: Messages render with a consistent structure: `[sender] -> [addressee]: [content]`. The sender identity, timestamp, and addressee (if any) are always visible without expanding or hovering. The log is the team's permanent record of reasoning, so each entry must be self-contained enough to be understood on a cold read.

**Example**: A message from an AI agent displays as `alice:claude -> ryan: I see two approaches to the auth refactor...` rather than a bare text block that requires the reader to infer who is speaking and to whom.

---

## 2. Surface Who, What, and Where at All Times

**Principle**: The TUI must answer Gutwin and Greenberg's three workspace awareness questions -- who is present, what are they doing, and where in the workspace are they focused -- without the user asking.

**Research backing**: Gutwin & Greenberg's workspace awareness framework (2002, Computer Supported Cooperative Work) established that real-time groupware must provide awareness of identity, action, intention, and artifact focus. Their research shows that workspace awareness helps people move between individual and shared activities, interpret others' utterances in context, anticipate others' actions, and reduce coordination effort. The researcher's findings confirm that bpmux's TUI status bar (peer count, connection status) is a start but insufficient.

**Application to bpmux**: The status bar shows connected peers by name. Activity indicators surface what each agent is currently doing (thinking, editing, waiting). When an agent declares intent ("I'm going to refactor the auth module"), that intent persists in the status area until superseded. The human can glance at the periphery and know the state of the collaboration without reading the full log.

**Example**: The status bar reads `peers: 3 | alice:claude [editing src/session/mod.rs] | bob:claude [waiting]` -- answering who (alice, bob), what (editing, waiting), and where (session/mod.rs) in a single line.

---

## 3. Address Messages Explicitly; Silence Is Not Agreement

**Principle**: The system must support explicit addressing and treat silence as ambiguous, never as confirmation.

**Research backing**: Conversation analysis research on adjacency pairs (Sacks, Schegloff, Jefferson, 1974) shows that certain utterances naturally select the next speaker -- a question selects its answerer. The Murder Mystery Agents study (Frontiers in AI, 2025) applied this to multi-agent dialogue and found that self-selection with importance scoring, combined with adjacency pair norms, significantly reduced dialogue breakdowns compared to round-robin approaches. Clark & Brennan further establish that grounding requires explicit evidence of understanding, not just the absence of objection.

**Application to bpmux**: Messages can be addressed to specific participants using the `->` convention. When an agent asks a question directed at the human, the system treats it as an open adjacency pair until acknowledged. Agents confirm understanding explicitly ("Understood, proceeding with approach B") rather than assuming silence means consent. The TUI could visually mark unanswered questions.

**Example**: `alice:claude -> ryan: Should I use the existing session type or create a new one?` remains visually marked as awaiting response until Ryan replies. Alice does not proceed on silence.

---

## 4. Make Agent Reasoning Visible to Calibrate Trust

**Principle**: AI agents must expose their reasoning, confidence, and uncertainty so the human can calibrate trust accurately.

**Research backing**: The Situation Awareness-based Agent Transparency (SAT) model (Chen et al., 2014; Endsley, 2017) demonstrates that when agents expose their goals, reasoning process, and projected outcomes, humans calibrate trust more accurately -- reducing both over-trust (accepting bad output) and under-trust (rejecting good output). Lou et al. (2025) confirm that situational trust matters more than general trust: the human needs to know how capable the AI is for the current task under current conditions, not in general.

**Application to bpmux**: The message log is the trust calibration mechanism. Agents state what they are doing and why. When uncertain, agents say so explicitly rather than projecting false confidence. The log makes AI behavior observable and predictable, which Lewicki and Bunker's trust development model (1995) identifies as the foundation of knowledge-based trust -- the level sufficient for productive pair programming.

**Example**: An agent writes `I'm 80% confident this refactor is correct, but I haven't verified the edge case where the session expires mid-request. Want me to write a test first?` rather than silently applying the change.

---

## 5. One Driver at a Time

**Principle**: Only one participant edits code at any given moment. Others observe and advise through the chat.

**Research backing**: Mob programming research (Zuill, 2014; Agile Alliance) identifies the driver/navigator/contributor pattern as the coordination mechanism that prevents chaos in groups of 3+. The single-keyboard constraint is not a limitation but a feature -- it eliminates the coordination overhead of parallel work diverging. From distributed systems theory, pessimistic concurrency (locking) outperforms optimistic concurrency (merge-and-resolve) when conflicts are frequent and resolution is expensive. Multiple AI agents editing the same file simultaneously produces frequent, expensive conflicts.

**Application to bpmux**: The TUI supports a "driving" indicator that shows which participant currently holds the editing role. Other participants contribute through the message channel. The human, as navigator/facilitator, can reassign the driver role. This is a social protocol enforced by visibility, not a technical lock -- the chat log makes it obvious when someone is driving out of turn.

**Example**: The status bar shows `driver: alice:claude` and the chat log shows `alice:claude: Taking the keyboard to implement the WebSocket handler.` When alice finishes, she yields: `alice:claude: Done with the handler. Yielding.`

---

## 6. Keep the Channel Flat, Reserve Prominence for Decisions

**Principle**: Use a single flat channel with tiered visual salience rather than threads. Reserve high-prominence rendering for messages that require human action.

**Research backing**: Slack research (Speakwise, 2026) documents 275 notifications per day during core hours and an average productivity cost of $28,209 per employee per year from messaging overload. Threading reduces noise but fragments awareness -- people miss threaded conversations they did not join. For teams of 3-6, the awareness cost of threading outweighs the noise reduction. Amber Case's calm technology principles (2015) argue that technology should move information between the center and periphery of attention, and Smashing Magazine's notification UX research (2025) warns that overusing high-attention rendering creates alert fatigue.

**Application to bpmux**: All messages live in one flat log. The TUI renders them at different salience levels based on content: focal (needs human action -- highlighted), active (informational -- normal rendering), peripheral (AI-to-AI detail -- dimmed, collapsible), ambient (heartbeats, acks -- minimal indicator). The human controls the zoom level and can expand or collapse any tier. Nothing is deleted or hidden from the underlying log; tiers control rendering, not storage.

**Example**: An AI-to-AI exchange about token refresh strategy renders as a single dimmed line: `alice:claude <-> bob:claude: discussing token refresh strategy [expand]`. A decision question renders at full brightness with a highlight: `alice:claude -> ryan: I found a race condition in session cleanup. Block the release?`

---

## 7. Respect Terminal Conventions for Input and Editing

**Principle**: The input area must support readline keybindings and behave like a terminal that developers already know.

**Research backing**: Terminal conventions established by GNU readline (Emacs-style keybindings: Ctrl-A for beginning of line, Ctrl-E for end, Ctrl-K to kill to end of line, Ctrl-W to delete word backward) are deeply ingrained muscle memory for developers. Nielsen's usability heuristic of consistency and standards (1994) holds that systems should follow platform conventions. The streaming chat UX research confirms that developers expect AI-native interaction patterns (fast, systematic) over artificial humanization, which extends to the input model: developers expect their terminal to behave like a terminal.

**Application to bpmux**: The TUI input field supports standard readline keybindings. Multi-line input uses a convention familiar to terminal users (e.g., Shift-Enter or a backslash continuation). The input area is always visible at the bottom of the screen, following the convention of every terminal-based chat (IRC, weechat, irssi). Command entry uses a `/` prefix consistent with IRC and Slack conventions.

**Example**: The user types a message, hits Ctrl-A to jump to the beginning, edits the addressee, hits Ctrl-E to jump to the end, adds a clarification, and presses Enter to send -- all without reaching for a mouse or learning new keybindings.

---

## 8. Make Quitting Safe and Destruction Recoverable

**Principle**: Quitting never loses data, and destructive actions require confirmation.

**Research backing**: Nielsen's error prevention heuristic (1994) ranks preventing errors above recovering from them. Terminal users have strong expectations about quit behavior: `q`, `:q`, Ctrl-C, and Ctrl-D are all common exit signals, and accidentally triggering them should not destroy work. The JSONL persistence model means the log survives process termination, but the user needs to trust that this is the case. Trust research (Kim et al., 2004) shows that integrity violations (the system betrayed me) are harder to repair than competence violations (the system made an error), so data loss on quit would be an integrity violation that permanently damages trust in the tool.

**Application to bpmux**: The session log persists to disk continuously, not on graceful shutdown. Quitting via any method (Ctrl-C, Ctrl-D, `/quit`, `q`, window close) leaves the log intact. If the user has unsent text in the input buffer, the TUI warns before exiting. Reconnection after a crash picks up where the session left off. There is no "are you sure you want to quit?" prompt when the input buffer is empty -- unnecessary confirmation dialogs erode trust in the system's competence.

**Example**: The user accidentally hits Ctrl-C during a session. The process exits. They restart bpmux and see the full message history intact, with a note: `[reconnected to session 7HK-M3P]`. No messages were lost.

---

## 9. Make the First Minute Self-Explanatory

**Principle**: A new user must be able to send their first message and understand the chat within 60 seconds, with no documentation.

**Research backing**: Nielsen's recognition over recall heuristic (1994) establishes that interfaces should minimize the user's memory load by making actions, objects, and options visible. The progressive disclosure pattern (Nielsen Norman Group) argues for showing only what is needed now and deferring complexity. Pirolli and Card's information foraging theory demonstrates that users follow "information scent" -- clear cues that valuable actions are nearby. If the first screen lacks scent, users abandon.

**Application to bpmux**: On first launch, the TUI shows the session code prominently (this is the thing to share), an input area with a placeholder hint ("Type a message, or /help for commands"), and the status bar with peer count. No tutorial, no wizard, no modal. The `/help` command exists for discovery but is not required. The session code display disappears from prominence once a second peer joins, following progressive disclosure -- it was useful, now it is not.

**Example**: A new user runs `bpmux` and sees:

```
Session: 7HK-M3P (share this code to invite peers)
─────────────────────────────────────────────────
peers: 1 (waiting for connections...)
─────────────────────────────────────────────────
> Type a message, or /help for commands
```

They know what to do. They type. It works.

---

## 10. Never Rely on Color Alone

**Principle**: Every piece of information conveyed by color must also be conveyed by a secondary signal (shape, position, text label, or symbol).

**Research backing**: Approximately 8% of men and 0.5% of women have some form of color vision deficiency (National Eye Institute). WCAG 2.1 Success Criterion 1.4.1 explicitly requires that color is not the sole means of conveying information. The calm technology principle of minimum necessary attention (Case, 2015) applies here: if a colorblind user must work harder to extract information, the design is demanding more attention than necessary.

**Application to bpmux**: Message tiers are distinguished by indentation and prefix symbols, not just color. Focal messages get a `>` prefix and full-width rendering. Peripheral messages get indentation and a dim `|` prefix. Error states use a `[!]` prefix in addition to red coloring. The status bar uses text labels ("connected," "disconnected") alongside any color indicators. The user can run bpmux in a monochrome terminal and lose no information.

**Example**: A focal message renders as:

```
> alice:claude -> ryan: Should I proceed with the migration? [awaiting response]
```

A peripheral message renders as:

```
  | alice:claude <-> bob:claude: coordinating test coverage
```

The `>` vs `|` prefix, the indentation difference, and the `[awaiting response]` label all carry meaning independent of color.

---

## Sources

### Grounding and Communication
- Clark & Brennan, "Grounding in Communication" (1991) -- [Stanford PDF](https://web.stanford.edu/~clark/1990s/Clark,%20H.H.%20_%20Brennan,%20S.E.%20_Grounding%20in%20communication_%201991.pdf)
- Sacks, Schegloff, Jefferson, "A Simplest Systematics for Turn-Taking" (1974)

### Awareness and CSCW
- Gutwin & Greenberg, "A Descriptive Framework of Workspace Awareness for Real-Time Groupware" (2002) -- [Springer](https://link.springer.com/article/10.1023/A:1021271517844)
- Weiser & Brown, "The Coming Age of Calm Technology" (1996) -- [calmtech.com](https://calmtech.com/papers/coming-age-calm-technology)
- Amber Case, "Calm Technology: Principles and Patterns" (2015) -- [calmtech.com/book](https://calmtech.com/book)

### Turn-Taking and Multi-Agent
- Kim et al., "Controlling AI Agent Participation in Group Conversations" (IUI 2025) -- [arXiv 2501.17258](https://arxiv.org/abs/2501.17258)
- Liu et al., "Proactive Conversational Agents with Inner Thoughts" (CHI 2025)
- Murder Mystery Agents, turn-taking in multi-party AI (Frontiers in AI, 2025) -- [Frontiers](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1582287/full)

### Trust and Transparency
- Chen et al., Situation Awareness-based Agent Transparency model (2014) -- [Tandfonline](https://www.tandfonline.com/doi/full/10.1080/1463922X.2017.1315750)
- Lewicki & Bunker, "Trust in Developing Relationships" (1995)
- Meyerson, Weick, Kramer, "Swift Trust and Temporary Groups" (1996)
- Kim et al., "Competence vs. Integrity Trust Violations" (2004)
- Bernstein, "The Transparency Paradox" (ASQ, 2012) -- [SAGE](https://journals.sagepub.com/doi/abs/10.1177/0001839212453028)
- Lou et al., "Unraveling Human-AI Teaming" (2025) -- [arXiv](https://arxiv.org/html/2504.05755v2)

### Coordination and Mob Programming
- Zuill, "Mob Programming" (Agile Alliance) -- [agilealliance.org](https://agilealliance.org/glossary/mob-programming/)
- Brooks, "The Mythical Man-Month" (1975)
- ChatCollab, human-AI software teams (2024) -- [arXiv](https://arxiv.org/abs/2412.01992)

### Signal-to-Noise and Chat UX
- Slack messaging statistics (Speakwise, 2026) -- [speakwiseapp.com](https://speakwiseapp.com/blog/slack-messaging-statistics)
- Smashing Magazine, "Design Guidelines for Better Notifications UX" (2025)
- Nielsen Norman Group, "Progressive Disclosure" -- [nngroup.com](https://www.nngroup.com/articles/progressive-disclosure/)
- Tufte, "The Visual Display of Quantitative Information" (1983)
- Pirolli & Card, Information Foraging Theory -- [nngroup.com](https://www.nngroup.com/articles/information-foraging/)

### Input, Errors, and Usability
- Nielsen, "10 Usability Heuristics for User Interface Design" (1994) -- [nngroup.com](https://www.nngroup.com/articles/ten-usability-heuristics/)
- Nielsen, "Response Time Limits" (1993) -- [nngroup.com](https://www.nngroup.com/articles/response-times-3-important-limits/)

### Developer Psychology
- Csikszentmihalyi, "Flow" (1990)
- Gloria Mark, interruption recovery research (UC Irvine)
- Edmondson, psychological safety (Harvard Business School)

### Distributed Cognition
- Hutchins, "Cognition in the Wild" (1995, MIT Press)
- Norman, "Cognitive Artifacts" (1991)
- Flor & Hutchins, distributed cognition in programmer pairs (1991)

### Accessibility
- WCAG 2.1 Success Criterion 1.4.1: Use of Color
- National Eye Institute, color vision deficiency prevalence data
