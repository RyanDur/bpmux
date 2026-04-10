# Arc Interaction Spec

*Designer spec for PM to shape into stories. How arcs work as a user-facing feature.*

## What the User Sees

### For a cold-starting agent (the primary use case)

An agent joins a bpmux session. It has CLAUDE.md and the last N messages. It wants to understand how this team works — not just the rules, but the judgment behind them.

**Interaction 1: Browse arcs**
The agent calls a tool (e.g., `list_arcs()`) and gets back a list of arcs with titles and weights:

```
1. "Handoff docs → log is the teacher" (weight: high, 7 lessons)
2. "Magic numbers → code speaks for itself" (weight: high, 5 lessons)
3. "Blue contrast → do the right thing simply" (weight: medium, 3 lessons)
4. "Role boundaries → features go through PM" (weight: medium, 2 lessons)
```

Arcs are sorted by weight (frequency × recency × source authority). The agent sees the most important arcs first.

**Interaction 2: Walk an arc**
The agent calls `read_arc(1)` and gets the reasoning path — the sequence of messages that produced the understanding. Not the full raw log, but the key messages with turns highlighted:

```
Arc: "Handoff docs → log is the teacher"
Lessons: [don't write handoff docs, test with experiments,
          log teaches culture, we are not the product, ...]

[1/12] Team writes handoff documents for Friday migration.
[2/12] Ryan: "are you leaving a note or teaching?" ← TURN
[3/12] Team deletes handoff docs, runs subagent experiments.
[4/12] Discovery: 80-message log teaches culture at 4.5/5.
...
[11/12] Ryan: "you are not the product" ← TURN
[12/12] Team discovers the arc concept itself.
```

The agent reads through the sequence and builds its own understanding of why things are the way they are.

**Interaction 3: Query by topic**
The agent faces a situation and wants to know if the team has dealt with it before:

```
query_arcs("how should I handle design feedback?")
→ Arc: "Role boundaries → features go through PM" (weight: medium)
  Turn: Ryan corrects designer for directing dev to remove a feature.
  Lesson: Designer specs UX, PM owns product decisions, dev owns code.
```

This is the reactive search layer — the agent knows what to ask.

### For the human owner

Ryan doesn't interact with arcs directly during normal work. Arcs are extracted from the conversation that already happens. Ryan's contributions to arcs are his corrections and questions — those become the turns.

At reflection points (retros, end of session), Ryan could be prompted: "These arcs emerged this session. Anything to add or correct?" But this is optional — arcs form from the conversation whether or not anyone reviews them.

## Minimum Data Model

An arc is:

```
{
  title: "Handoff docs → log is the teacher",
  messages: [msg_id_1, msg_id_2, ...],  // ordered sequence
  turns: [msg_id_2, msg_id_11],          // the pivotal messages
  lessons: ["don't write handoff docs", "test with experiments", ...],
  weight: {
    frequency: 3,    // reinforced by 3 other arcs
    recency: 0.9,    // recent (decays over time)
    source: "owner", // owner corrections weigh most
  },
  created: "2026-04-07T03:34:00Z",
  status: "active"   // active | superseded | retired
}
```

**Storage:** JSON file in the session directory alongside messages-current.jsonl. Human-readable, editable.

**Extraction:** Initially manual (team identifies arcs during retros). Later: automated detection of correction patterns (owner question → team response → behavior change).

## What This Is NOT

- Not threading — arcs span across time, not within a thread
- Not bookmarks — arcs have structure (sequence, turns, lessons, weight)
- Not summaries — arcs preserve the reasoning path, not just the conclusion
- Not tags — arcs are about sequences of messages, not individual message properties

## Open Design Questions

1. **How are arcs created?** Manual during retros? Semi-automatic (system suggests, human confirms)? Fully automatic?
2. **How do arcs age?** Weight decay over time? Explicit retirement? Supersession by newer arcs?
3. **Should walking an arc be interactive?** The research says active > passive. Could the system pause and ask "what would you do here?" before revealing the next message?
4. **How many arcs are enough?** Too few = gaps. Too many = noise (CBR utility problem). What's the right number for a team our size?
5. **Do arcs transfer across projects?** Ryan asked about networked learning stores. Cross-project arcs are the "teaching compounds" hypothesis.

## Phased Approach

**Phase 1: Manual arcs (smallest viable step)**
- Team identifies arcs during retros
- Arcs stored as JSON in session directory
- `list_arcs()` and `read_arc(id)` as bridge tools
- Cold-starting agents browse and walk arcs on join

**Phase 2: Query layer**
- `query_arcs(topic)` searches arc lessons and message content
- Returns relevant arcs ranked by weight and relevance
- Built on existing log search capabilities

**Phase 3: Automated detection**
- System detects correction patterns (owner question → behavior change)
- Suggests arcs for team review
- Team confirms or discards suggestions

**Phase 4: Interactive walking**
- System pauses during arc walk to ask reflective questions
- Walker's responses become part of the arc record
- Cognitive apprenticeship model: modeling → coaching → scaffolding → fading
