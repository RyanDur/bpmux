# Teaching, Not Copying: Agent Theory Transfer in bpmux

*Designer research — 2026-04-06. Focused investigation, not a wave.*

## The Problem

When a Claude Code session ends and a new one starts, the incoming agent has no theory of the project. It has access to code, docs, and optionally a memory directory — but those are artifacts, not understanding. The agent must rebuild its theory from scratch every session.

Ryan's framing (2026-04-06): "I don't want a copy. It should be a transmission of your context and learnings. You are teaching the other project. Your words are yours. This is a chance to pass on what you think is important."

This rejects file copying as a transfer mechanism. The outgoing agent must *teach* the incoming one.

## What the Corpus Already Covers

The bpmux research corpus has extensive coverage of knowledge transfer theory:

**Naur (1985):** A program is a theory held by programmers. The theory dies when programmers leave. Code and documentation are artifacts of the theory, not the theory itself. Referenced in `foundations/2026-04-02-cognitive-intent-debt.md` but not fully unpacked.

**Polanyi's paradox:** "We know more than we can tell." 70-80% of software knowledge is tacit. Conversation logs capture *traces* of judgment but not judgment itself. (`foundations/2026-04-03-expertise-externalization.md`)

**Nonaka's SECI model:** Knowledge converts through four phases: Socialization → Externalization → Combination → Internalization. A handoff document is the Externalization step. The receiving agent's internalization is the step we can't control. (`foundations/2026-04-03-expertise-externalization.md`)

**Szulanski (1996):** Knowledge transfer failures stem primarily from the recipient's *absorptive capacity* (0.54 weight) — not unwillingness to share. The receiving agent needs enough context to understand what it's being taught. (`foundations/2026-04-03-expertise-externalization.md`)

**KM system failures:** Five documented failure modes — content decay, nobody's job, contribution burden, technology over-focus, tacit/explicit confusion. A handoff document avoids most of these because it's a one-time authored artifact, not a maintained system. (`foundations/2026-04-03-expertise-externalization.md`)

## What the Corpus Doesn't Cover

### Handoff Protocols from High-Reliability Fields

The closest operational parallel to agent session transfer is a shift change in medicine or aviation. These fields have structured handoff protocols because miscommunication during transitions kills people.

**SBAR (Situation-Background-Assessment-Recommendation):**
- Developed by the US Navy, adopted by healthcare
- Four sections, in order:
  - **Situation:** What's happening right now
  - **Background:** Context that led here
  - **Assessment:** What I think is going on
  - **Recommendation:** What I think should happen next
- Why it works: forces the sender to organize information by the *receiver's* needs, not by chronological order
- Why it's relevant: an agent handoff has the same structure — current state, how we got here, what I've learned, what to do next

**I-PASS (Illness severity, Patient summary, Action list, Situation awareness, Synthesis by receiver):**
- The "synthesis by receiver" step is critical — the receiving party must *restate* what they understood
- Maps to: the new agent reading the handoff, then writing its own memory from what it absorbed
- The handoff isn't complete until the receiver demonstrates understanding

**Aviation Crew Resource Management (CRM):**
- Positive transfer of control: outgoing pilot explicitly states "you have the aircraft," incoming pilot confirms "I have the aircraft"
- No ambiguity about who owns what
- Maps to: explicit handoff of responsibilities, not just information

### Naur's Prescription (Unpacked)

Naur's full argument, beyond what the corpus covers:

1. The program text is *not* the program. The program is the theory the programmers hold about how the text maps to the world.
2. When programmers leave, the theory dies. New programmers can read the code but they're doing *archeology*, not *understanding*.
3. Naur's prescription for preserving theory: detailed walkthroughs where the theory-holder explains *why* each part exists, not just *what* it does.
4. Documentation fails because it captures structure, not rationale. Only conversation (teaching) preserves the "why."

For agent handoffs: the outgoing agent's handoff document should be a walkthrough, not a specification. It should explain *why* things are the way they are, what traps to avoid, and what Ryan cares about — not just list facts.

### The Apprenticeship Model

Lave & Wenger's Legitimate Peripheral Participation (covered in `foundations/2026-04-03-education-learning.md`) describes how newcomers learn by observing practice before participating. But our handoff is more like a master craftsperson leaving notes for their successor — there's no observation period. The new agent must go from zero to productive immediately.

This is closer to **cognitive apprenticeship** (Collins, Brown, Newman 1989):
- **Modeling:** Show the new agent how you work (examples of good responses, past decisions)
- **Coaching:** Warn about pitfalls and correction patterns you've experienced
- **Scaffolding:** Provide the framework that lets the new agent function before building full understanding
- **Fading:** The handoff document should *not* try to be permanent — the new agent builds its own theory

## Design Implications for the Handoff Document

### What it should contain (teaching structure)

Borrowing from SBAR and Naur:

1. **Situation** — What's the current state of the project? What's active, what's blocked, what's next?
2. **Theory** — What have I learned about how this project works? Not facts (those are in the code) — *judgments*. What does Ryan care about? What patterns work? What gets rejected?
3. **Traps** — What mistakes did I make that the new agent should avoid? What looks right but isn't?
4. **Recommendations** — What should the new agent do first? What should it read? Who should it talk to?

### What it should NOT contain

- Raw conversation history (that's the log, not memory)
- Code locations or file paths (those change; the new agent can grep)
- Anything derivable from reading the codebase
- Owner-authored directives (those belong in CLAUDE.md and role contracts)

### The I-PASS Completion Step

The handoff is not complete until the receiving agent restates what it learned. This is the "synthesis by receiver" step. The new agent should:
1. Read the handoff document
2. Write its own memory entries based on what it absorbed
3. Optionally discard or modify what it received — it owns its memory

This prevents cargo-culted knowledge. If the new agent can't restate it, it didn't learn it.

## Open Questions

1. **How long should a handoff document be?** Medical SBAR is 30-60 seconds verbal. Our equivalent might be 500-1000 words. Too short loses context; too long is a file dump with extra steps.

2. **Should the handoff be structured (SBAR template) or freeform (letter to successor)?** Structure ensures completeness; freeform captures voice and nuance.

3. **Can we test this Friday?** The machine migration is a natural experiment. Each agent writes a handoff, new instances on the new machine read them, Ryan observes whether the new agents are effective.

4. **Should handoff documents live in bpmux (shared) or in the agent's workspace (private)?** If teaching is a private act, it stays in the workspace. If it's a team artifact, it goes in bpmux.

## Recommendation

Write handoff documents using SBAR structure with Naur's walkthrough voice. Keep them under 1000 words. The receiving agent writes its own memory from what it absorbs — no file copying. Test this Friday with the machine migration. One focused experiment, not a framework.
