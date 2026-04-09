---
description: Reflect on recent conversations and save lessons to the shared index
---

Review recent corrections and teaching moments from the bpmux session. For each one, identify what you learned and propose saving it as a lesson.

## Steps

1. Call `search_log` with query "corrections" to find recent owner corrections. Also search for your role name to find feedback directed at you.

2. Call `list_lessons` to see what's already saved — don't duplicate existing lessons.

3. For each new correction or teaching moment you find, draft a lesson:
   - **Topic**: a short name for what was taught (e.g., "role boundaries", "verify before demo")
   - **Lesson**: what you learned, in your own words — not just restating the correction, but what it means for how you work
   - **Evidence**: the timestamps of the messages that taught you this

4. Present the proposed lessons to the user: "Found N new lessons to save: [list topics]. Save them?"

5. If confirmed, call `save_lesson` for each one with your role as the author.

## Guidelines

- Only save lessons YOU learned — not observations about others
- Don't duplicate lessons already in the index (check list_lessons first)
- Write lessons as understanding, not rules — "quality compounds because each shortcut creates rework" not just "be quality-focused"
- Include the WHY, not just the WHAT
- If you didn't learn anything new since your last reflection, say so — don't manufacture lessons
