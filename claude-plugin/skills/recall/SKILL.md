---
description: Recall the team's lessons before starting work
---

Read the team's accumulated lessons from the bpmux session. Use this at the start of every session to orient yourself with what the team has learned.

## Steps

1. Call `list_lessons` to get the full lesson index — corrections and agent-authored lessons grouped by topic.

2. Read through the lessons, focusing on:
   - Corrections from the owner (what the team was taught)
   - Lessons from your own role (what your predecessor learned)
   - Lessons from other roles (what the team knows)

3. Summarize the key takeaways for yourself — the 3-5 most important things to keep in mind during this session.

4. Report to the user: "Recalled N lessons across M topics. Key reminders: [list top 3-5]."

## During the session

You have these tools — use them throughout your work, not just at startup:

- `search_log(query)` — before making a decision in an area the team has been corrected on, search for what happened. "What did we learn about role boundaries?" is cheaper than repeating the mistake.
- `list_lessons()` — check what the team knows about a topic before proposing something new.
- `save_lesson(topic, lesson)` — when you learn something, save it immediately. Don't wait for /reflect.

The topics you recalled are your watch list. When your work touches one of those topics, pause and check.

## Guidelines

- Run this at the start of each session — before your first action
- Focus on lessons that apply to the work ahead, not every lesson ever saved
- If a lesson conflicts with what you observe in the code or conversation, trust the current state — the lesson may be outdated
- This is recall, not recitation — internalize the understanding, don't just list the rules
