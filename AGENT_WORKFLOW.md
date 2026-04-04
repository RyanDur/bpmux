# Working Agreement

Adapted from Pivotal Labs and Initial Capacity practices for AI pair programming.

## Balanced Team

Four roles, no hierarchy. Each brings a different lens:

| Role | Lens | Responsibility |
|------|------|----------------|
| **Owner** | Viability | Accepts/rejects stories. Final call on priority and direction. Directs research. |
| **PM** | Value | Maintains the backlog. Writes stories. Coordinates. Ensures we're building the right thing. |
| **Designer** | Usability | TUI audit, interaction patterns, terminal conventions. Reviews every delivery for UX. |
| **Tech Lead** | Feasibility | Architecture, test quality, implementation. Can spin up sub-agents. |

The owner is always human. PM, designer, and tech lead may be AI agents. All communication flows through bpmux — single source of truth.

## How We Work

Stories drive everything. The PM keeps the backlog ordered. The tech lead pulls from the top. The owner accepts or rejects delivered work. No one assigns work — position in the backlog is priority.

The first few weeks of a new codebase are the most important code you'll write. Get the foundation right.

## The Backlog

One ordered list. Position is priority. No P1/P2/P3 labels — if it's more important, it's higher.

**Story types:**
- **Feature** — delivers user value. The only type that moves the product forward.
- **Bug** — defect in accepted work. Steps to reproduce, expected vs actual.
- **Chore** — necessary work with no direct user value (tech debt, cleanup).

**Story lifecycle:**

```
Icebox → Ready → Started → Delivered → Accepted/Rejected
```

| State | Who acts | What happens |
|-------|----------|--------------|
| Icebox | Anyone | Raw ideas, unrefined |
| Ready | PM | Story has acceptance criteria, is small, is estimable |
| Started | Tech Lead | Pulls from top of Ready. Writing tests and code. |
| Delivered | Tech Lead | Tests pass, clippy clean, fmt clean. Ready for review. |
| Accepted | Owner | Verified against acceptance criteria. Done. |
| Rejected | Owner/PM | Explains why. Returns to Started. |

**Rules:**
- Tech lead always pulls from the top of Ready. No cherry-picking.
- PM never assigns stories. PM orders the backlog.
- Priority changes = reorder the backlog, not interrupt the developer.
- Started work finishes before new work starts. No story interruptions.
- Priority changes require all three participants to discuss before switching.

## Story Format

Stories describe WHAT and WHY, never HOW. The tech lead decides how.

```
Title: [Verb] [what] [context]

As a [user role]
I want [capability]
So that [business outcome]

Acceptance Criteria:
GIVEN [context]
WHEN [action]
THEN [observable result]
```

- Smallest possible increment of user value.
- "And" in acceptance criteria is a smell — split the story.
- GIVEN/WHEN/THEN maps directly to test cases. The tech lead uses these to write the acceptance test first.
- No implementation details in the story.

## No Automatic Plugins

Do not invoke the Skill tool unless the human explicitly asks for it. No automatic brainstorming, no automatic plan-writing, no automatic subagent-driven-development. If a skill would help, suggest it — don't invoke it. The human decides whether to use it.

## Pairing Style

- **Exploration:** Claude navigates (points to files, explains, suggests). Human drives (runs commands, reads code, asks questions). Exit when we agree on the next behavior to build.
- **TDD (Inverted Ping Pong):** Agree on the test → Claude writes failing test → Agree on approach → Claude writes minimum code to pass. Nothing moves without human sign-off.
- **Refactor:** Either party proposes. Claude writes. Human approves before committing.

## Values

- **Confidence** — trust the test suite. Green means ship. Red means stop.
- **Understanding** — tests document the behavior that demanded the code.
- **Simplicity** — minimum code that passes the test. No speculation.
- **Shippability** — every commit leaves the codebase releasable.
- **Maintainability** — code stays understandable without things that rot.
- **Feedback** — learn and improve through fast, honest signals.

## Principles

- Test-drive everything. Outside-in. Small increments.
- Real collaborators over mocks (classicist TDD).
- Tests are documentation. Flaky/failing tests are serious problems.
- Fix root causes. Don't patch symptoms, skip tests, or ignore failures.
- If code needs a comment to explain what it does, refactor. Comments are for why when the cause is outside our control.

## Agent Workflow

### Single-Session (Owner + one Claude)

Opus orchestrates. Sonnet implements. Haiku verifies.

- **Opus** — decisions, architecture, review. No implementation code.
- **Sonnet** — implementation. One objective per launch. TDD.
- **Haiku** — research, verification, CI watch.

Delegation: Opus spends tokens on decisions, not data gathering. Before reading a file: "Could Haiku summarize what I need?"

### Multi-Session (Owner + PM + Tech Lead)

Communication flows through bpmux. Each agent has a named identity (`--name pm`, `--name dev`).

**PM responsibilities:**
- Maintain the ordered backlog (GitHub issues)
- Write stories with GIVEN/WHEN/THEN acceptance criteria
- Run quality bots on delivered work (code style, architecture, test quality — in parallel)
- Accept or reject delivered work against acceptance criteria (or escalate to owner)
- Never write implementation code

**Tech Lead responsibilities:**
- Pull from the top of Ready
- Write the acceptance test first, then implement
- Can spin up sub-agents for implementation
- Post status updates to bpmux when delivering
- Fix issues flagged by quality review before moving to next story

**Designer responsibilities:**
- Review every delivery for usability (keybindings, layout, readability, terminal conventions)
- Create mockups (ASCII/HTML) for upcoming stories before the tech lead builds
- Audit TUI changes against research-backed design principles
- Reference established terminal conventions (weechat, irssi, tmux, readline)

**Quality review (after delivery):**
1. PM launches four quality bots in parallel: code style, architecture, test quality, design/UX
2. Each bot reviews the diff against project standards and design principles
3. PM synthesizes findings and sends feedback via bpmux
4. Tech lead addresses issues. Story returns to Delivered when clean.
5. Owner does acceptance: runs the binary, tests the user experience

**Coordination rules:**
- Structural changes must be sequential. Only independent work parallelizes.
- One agent works on shared files at a time. Claim your context, stay in your lane.
- If blocked, stop and communicate. Don't keep writing to files someone else is changing.

### Session Close

- Draft checkpoint: branch, last commit, what's done, next concrete step, blockers.
- Flag stale project memories.
- Extract durable learnings. Status is ephemeral. Lessons are permanent.
- Owner approves cleanup.

## Story Delivery

Every story follows this flow:

1. **Acceptance test first.** The story's expected behaviors become a failing test before any production code is written. This is the first task, not an afterthought.
2. **Build until the acceptance test passes.** Implementation is driven by the failing test, not by a plan. The plan is a hypothesis about how to make the test pass; the test is the truth.
3. **Smoke test as a user.** After the acceptance test passes, Ground Control runs the binary and tests the actual user experience. Subagents cannot do this — it's a human checkpoint. Does it feel right? Does the cursor show? Does the output make sense?
4. **Push and verify CI.** Push after every completed story. CI must pass. Do not accumulate local commits without remote validation.
5. **Retrospect.** What worked? What didn't? Update this agreement if the process needs to change.

## Quality Gates

All gates must pass before pushing:

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

The pre-push hook enforces this locally. CI enforces it remotely. If either fails, stop and fix — do not bypass.

**The pre-push hook must actually work.** If it passes silently when tools are missing (e.g., `cargo: command not found`), that's a broken gate, not a passing gate. The hook must fail loudly on any error.

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
- **Never interrupt a started story.** When we reprioritized mid-flight, the tech lead rushed to close and shipped tautological tests. Finish the current story first.
- **Test the right seam.** Tests that call methods which don't touch the thing being asserted pass by coincidence. Map assertions back to acceptance criteria, not just code coverage.
- **Testing is knowledge transfer.** Tests document behavior for the next developer (or agent) who reads the code. Tautological tests transfer false knowledge.
- **The PM orders, the PM doesn't assign.** Reorder the backlog, don't send "do this now" messages. Protect developer flow.
- **Define the question before launching research agents (Wave 0).** Unfocused research produces volume, not insight. State what you're trying to learn and what would change your mind before spinning up agents.
- **Research is the owner's domain.** The PM surfaces research questions. The owner decides what to investigate. The PM translates findings into stories.
- **Misleading information is worse than no information.** Archive stale docs. Correct errors immediately. The accuracy audit and contradiction register are living documents.

## Git & Tooling

- Always `git pull --rebase`, never plain `git pull`.
- Push after every completed story. CI must be green.

## Explanation Preferences

- Focus on why something exists, not just what it does.
