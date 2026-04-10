# Lesson Index Design

PM design doc — 2026-04-08. Captures the architecture discussion between PM, dev, and designer.

## What it is

A pre-built index of what the team has learned, with pointers back to the evidence in the log. Agents query the index instead of scanning the full log. The index grows from real usage.

## Why

A cold-starting agent shouldn't read 15K messages to understand how the team works. The index answers "what has this team learned about X?" instantly, with evidence links for deeper context.

## Architecture

Three layers, each independent:

### Layer 1: Classification (built — #107)
- Rule-based classifier tags messages as correction/discussion/decision
- Runs locally, zero token cost
- Tags are computed in real-time as messages arrive

### Layer 2: Correction pointers (to build)
- When the classifier tags a correction, store a pointer in the index
- Pointer: `{timestamp, topic_keywords, evidence: [msg_ids]}`
- Keywords extracted from message text (no LLM needed)
- Stored in `lessons.json` in the session directory
- Runs in background, zero LLM cost

### Layer 3: Lesson summaries (to build)
- On first query for a topic, pull matching correction clusters
- Ask the agent's LLM to summarize the lesson
- Cache the summary in `lessons.json`
- One-time LLM cost per topic, zero cost on subsequent queries
- Summaries include: topic, lesson text, evidence pointers, established date, reinforcement count

## Data model

```json
{
  "corrections": [
    {
      "id": "2026-04-07T02:35:48Z",
      "keywords": ["role", "boundaries", "designer", "pm"],
      "evidence": ["msg_ts_1", "msg_ts_2", "msg_ts_3"],
      "classified_as": "correction"
    }
  ],
  "lessons": [
    {
      "topic": "role boundaries",
      "summary": "Design feedback goes through PM. Designer proposes, PM prioritizes, dev builds.",
      "evidence": ["msg_ts_1", "msg_ts_2"],
      "established": "2026-04-07",
      "last_reinforced": "2026-04-08",
      "reinforcement_count": 3,
      "revised_by": null
    }
  ]
}
```

## Trigger model

- **Background (zero cost):** classifier tags corrections → store pointers
- **On first query (one-time LLM cost):** pull matching clusters → LLM summarizes → cache
- **On subsequent queries (zero cost):** return cached summary + evidence

## Correction and retirement

- Lessons have `last_reinforced` and `reinforcement_count`
- New corrections on the same topic update `last_reinforced` and may update the summary
- Contradicting corrections add a `revised_by` pointer — the later lesson supersedes
- Nothing gets deleted — the revision history is the arc
- Stale lessons (not reinforced recently) naturally lose weight in query results

## Session restart

- `lessons.json` persists in session directory
- On startup: load the index, process new messages since last update
- Incremental, not full rebuild

## Open questions

1. **Who extracts topic keywords?** Simple: split on whitespace, remove stop words. Better: TF-IDF on the correction cluster. Best: the classifier learns which words predict topics.
2. **How does topic matching work?** Keyword overlap? Fuzzy matching? This affects whether "role boundaries" matches a correction about "designer going through PM."
3. **Should lessons be editable by the owner?** Ryan said he wants to see and correct the index. A `/lesson correct` command could update summaries.
4. **How many lessons before the index needs structure?** JSON is fine for tens of lessons. Hundreds might need indexing. Thousands need a database. What's the expected scale?

## What to build first

A story that covers Layer 2 (correction pointers) only. Layer 3 (LLM summaries) is a separate story. Keep them independent so each can be built, tested, and accepted separately.
