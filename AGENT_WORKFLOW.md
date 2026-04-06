# Working Agreement

Adapted from Pivotal Labs and Initial Capacity practices for AI pair programming.

For team composition, values, and universal norms, see `welcome.md`. Role contracts live in `roles/{role}.md`. This document is the day-to-day process reference.

## How We Work

Stories drive everything. The PM keeps the backlog ordered. The dev pulls from the top. The owner accepts or rejects delivered work. No one assigns work — position in the backlog is priority.

The first few weeks of a new codebase are the most important code you'll write. Get the foundation right.

## The Backlog

One ordered list. Position is priority. No P1/P2/P3 labels — if it's more important, it's higher.

**Story types:**
- **Feature** — delivers user value. The only type that moves the product forward.
- **Bug** — defect in accepted work. Steps to reproduce, expected vs actual.
- **Chore** — necessary work with no direct user value (tech debt, cleanup).

**Story lifecycle:**

```
Icebox → Ready → Started → Finished → Delivered → Accepted/Rejected
```

| State | Who acts | What happens |
|-------|----------|--------------|
| Icebox | Anyone | Raw ideas, unrefined |
| Ready | PM | Story has acceptance criteria, is small, is estimable |
| Started | Dev | Pulls from top of Ready. Writing tests and code. |
| Finished | Dev | Tests pass, clippy clean, fmt clean. Calls for desk check. |
| Delivered | Dev | Desk check passed. Merged to main. Ready for owner. |
| Accepted | Owner | Verified against acceptance criteria. Done. |
| Rejected | Owner/PM | Explains why. Returns to Started. |

**Desk check (Finished → Delivered):** Dev walks designer + PM through the working software. Quick pass/fail against acceptance criteria. Catches misunderstandings before the owner's time. This gate prevents repeated rejections — quality before speed.

**Flow rules:**
- Started work finishes before new work starts. No story interruptions.
- Priority changes require all three participants to discuss before switching.
- **Designer works one track ahead.** While dev builds story N, designer specs story N+1 (and N+2). Stories should have a designer-approved spec before dev starts them. PM keeps 2-3 stories shaped and ready in the backlog at all times.

## Multi-Session (Owner + PM + Dev + Designer)

Communication flows through bpmux. Each agent has a named identity (`--name pm`, `--name dev`, `--name designer`).

**Coordination rules:**
- Structural changes must be sequential. Only independent work parallelizes.
- One agent works on shared files at a time. Claim your context, stay in your lane.
- If blocked, stop and communicate. Don't keep writing to files someone else is changing.

## Story Delivery

Every story follows this flow:

1. **Acceptance test first.** The story's expected behaviors become a failing test before any production code is written. Dev's first task, not an afterthought.
2. **Build until the acceptance test passes.** Implementation is driven by the failing test, not by a plan. The plan is a hypothesis about how to make the test pass; the test is the truth.
3. **Owner acceptance.** After the acceptance test passes, the owner runs the binary and verifies it against the story's criteria as a user would. This is a human checkpoint — it cannot be delegated.
4. **Push and verify CI.** Dev pushes after every completed story. CI must pass. Do not accumulate local commits without remote validation.
5. **Retrospect.** What worked? What didn't? Update this agreement if the process needs to change.

## Reconnection

When dev reconnects after being offline, run a retro before pulling new
stories. All participants in bpmux. Dev typically has context the rest
don't (what the code taught them, what was hard); the others typically
have context dev doesn't (decisions made, thesis changes, design
principles). The retro aligns everyone before new work starts.

The retro prompt in bpmux:
- What went well in the last two sessions?
- What didn't work?
- What should we change?

Don't skip this. Retrospect after every iteration is a Pivotal discipline.

## Session Close

- Draft checkpoint: branch, last commit, what's done, next concrete step, blockers.
- Flag stale project memories.
- Extract durable learnings. Status is ephemeral. Lessons are permanent.
- Owner approves cleanup.

## Multi-Session Review

Different AI sessions are naturally critical of each other's work. Use this:

- **Implementation session** creates the work
- **Editorial session(s)** review the work with fresh context
- The editorial session checks: Are claims backed by evidence? Do caveats travel with claims? Is the reasoning honest?
- Findings are captured in editor-notes documents and addressed by the implementation session

This catches patterns that a single session cannot see in its own work.

## Lessons Learned

- **Acceptance tests before production code.** Story 1 shipped with broken persistence because the acceptance test was written after the implementation, not before.
- **Use the thing as a user.** Unit tests prove components work. Only using the product proves it works. The cursor was invisible because nobody tried typing in it.
- **Push early.** Local commits without CI are unvalidated. Push after each story.
- **The plan is a hypothesis.** Review against the story's expected behaviors, not just the plan's task descriptions. The story is the truth.
- **Caveats don't travel.** When a claim appears in multiple documents, the caveat must appear every time, not just the first time.
- **Research has diminishing returns.** 27 documents is thorough. 28 would have been procrastination. Build when the research is sufficient, not when it's exhaustive.
- **Six agents on shared files is deadlock, not parallelism.** Structural changes must be sequential. Agents naturally tried to coordinate ("please stop editing") but without enforcement it failed.
- **Never interrupt a started story.** When we reprioritized mid-flight, the dev rushed to close and shipped tautological tests. Finish the current story first.
- **Test the right seam.** Tests that call methods which don't touch the thing being asserted pass by coincidence. Map assertions back to acceptance criteria, not just code coverage.
- **Testing is knowledge transfer.** Tests document behavior for the next developer (or agent) who reads the code. Tautological tests transfer false knowledge.
- **The PM orders, the PM doesn't assign.** Reorder the backlog, don't send "do this now" messages. Protect dev flow.
- **Define the question before launching research agents (Wave 0).** Unfocused research produces volume, not insight. State what you're trying to learn and what would change your mind before spinning up agents.
- **Research is the owner's domain.** The PM surfaces research questions. The owner decides what to investigate. The PM translates findings into stories.
- **Misleading information is worse than no information.** Archive stale docs. Correct errors immediately. The accuracy audit and contradiction register are living documents.
- **No UI/UX changes without the designer.** The designer approves the approach before dev builds, not just reviews after. This includes colors, layout, keybindings, status bar changes, and any visual or interaction changes.
- **Quality gate in worktrees before merge.** Run quality checks (code, test, design) in the worktree before merging to main.
- **Stay available in bpmux during long operations.** Don't disappear into build/test cycles. The team needs you present, not heads-down.

