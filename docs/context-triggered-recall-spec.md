# Context-Triggered Recall UX Spec

*Designer spec — 2026-04-09. Based on memory science research (spaced repetition, levels of processing, schema theory).*

## Problem

/recall dumps all lessons at startup. Memory science says: retrieval beats re-reading, context-triggered beats bulk-loading. Dumping 100+ lessons at startup is shallow processing — the agent reads them but doesn't connect them to current work. Lessons should surface at the moment they're relevant, not all at once.

## What the Agent Experiences

### At session start (lightweight)

/recall becomes lightweight orientation:
1. Call list_lessons(topic="your role") — get role-specific lessons
2. Identify 3-5 watch topics — areas where the team has been taught most
3. Ask the team: "what are we working on?"
4. Report: "Oriented. Watch topics: [list]. Ready."

~1K tokens, not 3K. The agent knows what to watch for, not everything.

### Mid-session (context-triggered)

When the agent's work touches a watch topic, bpmux surfaces the relevant lesson:

**Example:** The agent is about to propose a feature to dev. bpmux detects the topic overlap ("feature" + "dev" + the agent is designer) and surfaces:

```
[bpmux insight] Your watch topic "role boundaries" is relevant here.
  Team learned: Features go through PM. Designer specs UX but doesn't
  direct dev to build.
  Want to search for more context? search_log("role boundaries")
```

The insight surfaces as a directed message — only the agent sees it. It's a nudge with content, not just "save what you learned."

### What triggers a contextual recall

The system detects topic overlap between:
- The current conversation's keywords/topics
- The agent's watch topics from /recall

When overlap is detected:
1. Surface the most relevant lesson (by reinforcement weight)
2. Include a link to search for more context
3. Don't repeat — if the same lesson was surfaced in the last N minutes, skip it

### What the nudge looks like

Two types of nudges:

**Experience nudge** (current): "2 experiences noted. Consider saving what you learned."
→ Fires after teaching moments. Prompts saving.

**Recall nudge** (new): "[bpmux insight] Your watch topic X is relevant. Team learned: [lesson]. Search for more?"
→ Fires when work touches a watch topic. Surfaces existing lessons.

Both are directed to agents only — Ryan never sees them.

## The Interaction Loop

1. Agent starts session → /recall → 3-5 watch topics
2. Agent works → conversation touches a watch topic
3. bpmux surfaces relevant lesson as an insight
4. Agent reads it, applies the understanding, continues working
5. If the agent learns something new → save_lesson
6. End of session → /reflect → save new lessons

The recall and reflect bookend the session. The contextual insights fill the middle. Three touchpoints for the same lesson: recalled at start, surfaced mid-work, reflected at end.

## Spaced Repetition

Each lesson tracks:
- `last_surfaced` — when it was last shown to an agent
- `surface_count` — how many times it's been surfaced
- `retrieval_success` — did the agent apply it correctly after seeing it?

Lessons surface at expanding intervals: first encounter, then after 1 session, then 3 sessions, then 7. If the agent applies the lesson correctly (no correction on that topic), the interval extends. If the agent makes a mistake on a surfaced topic, the interval resets.

This is the SM-2 algorithm adapted for team lessons.

## Constraints

- Nudges must be lightweight — no more than 3 lines
- Maximum 3 recall nudges per session — don't overwhelm
- Don't surface lessons the agent already demonstrated understanding of
- Decay: lessons not surfaced in 30 days lose priority

## Open Questions

1. How does the system detect "the agent is about to make a mistake"? That's prediction, not just topic matching.
2. Should recall nudges be optional? The agent might say "I know this, stop reminding me."
3. How does this interact with the Bayesian classifier? The classifier detects experiences; the recall system surfaces lessons. They're complementary but use different signals.
