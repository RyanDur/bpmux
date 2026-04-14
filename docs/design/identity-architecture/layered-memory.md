# Layered memory model

*Four layers of memory at different stability levels, each referencing the layer below.*

## The layers

```
┌─────────────────────────────────────────────────────────────┐
│                     IDENTITY FILE                            │
│              ~/.bpmux/identity/{role}.json                   │
│                                                              │
│  values: { clarity, honesty, craft, individuality }          │
│  principles: [ { name, serves, description } ]               │
│                                                              │
│  ● most stable — changes at retro time                       │
│  ● portable across projects (no client IP)                   │
│  ● loads at every session start                              │
│  ● the "who am I" layer                                      │
├──────────────────────────┬──────────────────────────────────┤
│                          │ principles reference arcs         │
│                          ▼                                   │
│                      ARC INDEX                               │
│              ~/.bpmux/arcs/{role}.json                       │
│                                                              │
│  arcs: [ {                                                   │
│    summary: "the #191 saga",                                 │
│    beginning: "context coherence broken",                    │
│    branches: [ { middle, ends, learnings } ],                │
│    refs: { session: "2026-04-12", commits: [...] },          │
│    principles_informed: [ "two-audience", "verify" ]         │
│  } ]                                                         │
│                                                              │
│  ● moderately stable — grows as experiences accumulate       │
│  ● project-scoped (references client-specific events)        │
│  ● the "what shaped me" layer                                │
├──────────────────────────┬──────────────────────────────────┤
│                          │ arcs reference lessons             │
│                          ▼                                   │
│                    LESSON INDEX                               │
│              lessons.json (per session)                       │
│                                                              │
│  lessons: [ {                                                │
│    topic: "two-audience",                                    │
│    lesson: "both audiences in every AC",                     │
│    reinforcement: 5,                                         │
│    arc_ref: "#191-saga"                                      │
│  } ]                                                         │
│                                                              │
│  ● operational — reinforced or decays per session            │
│  ● session-scoped                                            │
│  ● the "what I've learned recently" layer                    │
├──────────────────────────┬──────────────────────────────────┤
│                          │ lessons reference raw events       │
│                          ▼                                   │
│                   SESSION LOG                                │
│           messages-current.jsonl + git log                   │
│                                                              │
│  raw conversation, commits, captures, decisions              │
│                                                              │
│  ● ephemeral — current session only                          │
│  ● the "what actually happened" layer                        │
│  ● the "documents" ryan referenced                           │
└─────────────────────────────────────────────────────────────┘
```

## Progressive disclosure on identity

```
SHALLOW                                                   DEEP
   │                                                        │
   ▼                                                        ▼

 "verify         "learned from        "on 2026-04-12,     full session
  through         the #191 saga        dev claimed         log with
  direct          where display-       'I pinged pm'       timestamps,
  observation"    only desk-check      based on a tool     captures,
                  missed the           ack, but the        git diffs
   principle      delivery gap"        message never
                                       arrived..."
                   arc summary
                                        raw event
```

Each zoom level adds detail without requiring ALL detail at once. the reader starts with the principle, drills to the arc if they want the story, drills to the raw events if they want the evidence.

## What lives where

| Content | Layer | Persists across | Example |
|---|---|---|---|
| Values | Identity | projects, sessions, engagements | "clarity" |
| Principles | Identity | projects, sessions | "work outside-in" |
| Arc summaries | Arc index | sessions (within project) | "the #191 saga" |
| Learnings | Lesson index | session (with reinforcement) | "both audiences in every AC" |
| Raw events | Session log | current session only | messages, commits, captures |

## Graduation path

```
raw event ──(classifier detects)──→ learning ──(reinforced across arcs)──→ principle ──(serves a value)──→ identity
```

- A learning that fires in ONE arc is a lesson.
- A learning that fires across MULTIPLE arcs becomes a principle candidate.
- A principle that consistently serves the same WHY reveals a value.
- Values, once named, guide which new arcs to pay attention to (top-down).
