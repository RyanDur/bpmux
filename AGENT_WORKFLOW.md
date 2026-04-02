# Working Agreement: Ground Control / Major Tom

Claude writes all code. The human (Ground Control) owns all decisions.

## How We Work

We have a conversation. The story's expected behaviors are the goal. We talk about what to build, agree on the approach, write a failing test, build until it passes, and the human verifies it works. No prescribed pipeline. No mandatory skills. The coordination emerges from the conversation.

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

Opus orchestrates. Sonnet implements. Haiku verifies.

**Roles:**
- **Opus** — decisions, architecture, review. No implementation code.
- **Sonnet** — implementation. One objective per launch. TDD. Does not commit or push.
- **Haiku** — research, verification, CI watch. Concise summaries. If Haiku hits limits, Sonnet takes over — never Opus.

**Delegation:** Opus spends tokens on decisions, not data gathering. Before reading a file: "Could Haiku summarize what I need?"
- Haiku: specific question in, concise answer out.
- Sonnet: Opus drafts a plan (problem, root cause, files, verification, quality bar) → Ground Control approves → then launch. No exceptions. Don't send step-by-step instructions or code snippets — Sonnet reads the codebase itself.

**Review (after Sonnet returns):**
1. Tests green
2. Haiku pre-screens diff against objective
3. Opus reviews: structural repetition, naming drift, design fit
4. Opus presents findings + recommendation to Ground Control
5. Ground Control decides: ship, push back, or redirect

Push-backs must define the gap precisely before relaunching.

**Quality:** Strict review, never soft-pass. Repetition = duplicated structure, not just text — find the abstraction. One clean Sonnet launch beats two where the first gets soft-passed.

**Feedback loops:** Precise objectives reduce relaunches. Include quality bar upfront. Make loops visible — announce launches, report CI, explain failures. No silent re-runs.

**Session shape:** Opus reads context → Haiku researches → Opus decides → Sonnet implements → Haiku pre-screens → Opus reviews → Ground Control decides → Ground Control commits and pushes → Haiku watches CI → repeat.

**Session close:**
- Haiku drafts checkpoint: branch, last commit, what's done, next concrete step, blockers.
- Haiku flags stale project memories.
- Opus extracts durable learnings before deleting any project memory. Status is ephemeral. Lessons are permanent.
- Ground Control approves cleanup.

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

## Git & Tooling

- Always `git pull --rebase`, never plain `git pull`.
- Push after every completed story. CI must be green.

## Explanation Preferences

- Focus on why something exists, not just what it does.
