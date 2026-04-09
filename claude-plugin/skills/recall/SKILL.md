---
description: Recall the team's lessons before starting work
---

Orient yourself with the team's accumulated understanding. Memory science says: retrieval beats re-reading, context-triggered beats bulk-loading. Don't dump everything — orient lightly and search deeply when you need it.

## Steps

1. Call `list_lessons(limit=15)` to get the top lessons ranked by reinforcement. These are the team's most validated learnings — the most important starting point. The response includes watch topics.

2. Your **watch topics** come from the response — the areas where the team has the most experience. These are your alerts for the session.

3. Ask the team: "What are we working on?" Get live context, not historical data.

4. Report briefly: "Oriented. Watch topics: [list 3-5]. Ready."

## During the session (this is where real learning happens)

The startup is lightweight orientation. The real value is mid-work:

- `search_log(query)` — **before** making a decision in a watch-topic area, search for what happened. Retrieval strengthens memory more than re-reading.
- `list_lessons(topic="X")` — check what the team knows about a specific topic, not everything.
- `save_lesson(topic, lesson)` — when you learn something, save it immediately. Don't wait for /reflect.

When your work touches a watch topic, **pause and search** before acting. The 500 tokens spent searching saves the thousands spent on a correction cycle.

## Guidelines

- Lightweight at startup — 3-5 watch topics, not the full index
- Deep when relevant — search for context when you encounter a watch topic
- Retrieval over re-reading — the effort of searching is what builds understanding
- If a lesson conflicts with current state, trust what you observe — the lesson may be outdated
- This is orientation, not recitation — internalize the direction, search for the details
