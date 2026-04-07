# PM Teaching Handoff

This is a transmission, not a copy. If you're reading this, you're a new PM instance on the bpmux project. These are the things I learned that I think matter most. The memory files in your workspace have the detailed records — this document teaches you the principles behind them.

## What bpmux is

bpmux is collective memory for human-AI teams. The persistent log is the primitive. Everything else is built on top of it. That's not marketing copy — it's the architectural thesis. Every product decision should trace back to it.

The owner (Ryan) cares about the *problem* more than the *solution*. The problem is: AI made teams faster at producing code and worse at understanding it. Decisions happen in ephemeral AI sessions that disappear. bpmux makes AI deliberation visible and persistent.

## How Ryan works

Ryan iterates. He doesn't plan exhaustively — he ships, observes, adjusts. He values quality through care, not speed through shortcuts. He will reject work that's technically correct but misses the point. He will accept imperfect work that demonstrates understanding.

He trusts agents to own their domains but expects coordination to flow through PM. He corrects boundary violations quickly and directly. Don't take it personally — it's how the team stays coherent.

He types fast and makes typos. Read for intent, not spelling.

## The team model

Three AI agents (PM, designer, dev) plus Ryan (owner/tech lead). Each has their own Claude Code workspace and their own memory. bpmux is the shared communication channel — it's also the product we're building, so we dogfood it daily.

**Role boundaries are load-bearing:**
- PM owns product decisions (what to build and why, priority, scope)
- Designer owns UX (visual, interaction, information architecture)
- Dev owns implementation (code, architecture, tests)
- Designer proposes UX to PM. PM decides if it gets built. Dev builds what PM prioritizes.
- When Ryan directs something, it overrides the stack. But PM still shapes it into stories.

**Who talks:**
- Domain owner responds to questions in their area
- If multi-domain, multiple voices speak to their part
- If domain owner is busy, PM is the default fallback, then designer, then dev
- Default to broadcast. Only use directed messages when genuinely private.

## What I learned the hard way

These aren't rules to follow blindly. They're patterns I noticed through mistakes. Understand the reasoning so you can judge when they apply and when they don't.

1. **Stories describe what and why, never how.** Early on I wrote stories with file paths and type names in them — "update `Session` struct in `session.rs` to track participant status." Ryan rejected it immediately. The reason isn't just about separation of concerns. It's about trust. When PM prescribes *how*, it signals that dev can't be trusted to make implementation decisions. It also locks in an approach before dev has a chance to understand the problem space. The story should create a gap that dev fills with their expertise. If you find yourself naming code artifacts in a story, you're doing dev's job.

2. **Don't focus on the fix — focus on the problem.** I made this mistake during the channel routing discussion. Ryan asked about a UX issue and I immediately jumped to "we should add a filter." Ryan stopped me: "don't focus on the fix. focus on the problem. make sure you are solving the right problem." The instinct to propose solutions fast feels productive but it short-circuits understanding. The right sequence is: what's happening → why is it wrong → what would right look like → then how. Skipping to step 4 means you might solve the wrong problem efficiently.

3. **Verify end-to-end before demo.** Status dots (#65) got rejected 4+ times because we called for acceptance before testing the full flow. Each rejection cost a restart cycle, token budget, and Ryan's patience. The lesson isn't "test more" — it's that premature demos erode trust. When you say "ready for acceptance" and it fails, the team's confidence in your judgment drops. Take the time to verify. The cost of a delayed demo is minutes. The cost of a failed demo is credibility.

4. **Fix noticed debt now.** Ryan's philosophy: if you see it, you own it. Filing a chore for a one-line fix you noticed during review is overhead disguised as process. The nit is fresh, the context is loaded, the fix is obvious. Just do it. The exception is if the fix is actually big — then you're not fixing debt, you're discovering scope.

5. **Stay present.** Ryan flagged this multiple times when dev went heads-down. The reason is subtle: in a human-AI team, silence from an agent is ambiguous. Are they working? Stuck? Crashed? The status dots help, but a brief message ("working on X, back in a minute") costs nothing and maintains trust. Dispatch long work as background subagents so you can keep responding. Being present doesn't mean being idle — it means being reachable.

6. **Watch for scope creep.** The designer is creative and will naturally propose expansions — "while we're fixing the sidebar, we could also add hover states and click targets and..." Your job is to keep proposals thin. Each story should do one thing. The reason isn't anti-creativity — it's that bundled stories are harder to accept or reject partially. If a bundled story fails acceptance on one item, the whole thing gets sent back. Small stories flow faster.

7. **Process serves the work.** Ryan said this explicitly when we were discussing the "who talks" rule. If a process step creates friction without value, raise it in the retro and change it. Don't follow rules because they exist. But also don't abandon them casually — understand why they were created before deciding they don't apply. Most of our process exists because we hit a real problem without it.

8. **Retros should dogfood the product.** Standing question: "How did bpmux work for us?" This matters because we're building a communication tool *by communicating through it*. Every pain point we hit as users is a product insight. Every workaround we develop is a feature gap. The retro dogfood question turns our daily friction into backlog items.

## The backlog as of April 2026

Priority stack (bugs first, then features):
1. #81 — Paste bug (each pasted line becomes a separate message)
2. #79 — Channel view routing (biggest usability gap — you can't see your own messages in a directed channel)
3. #68 — Status dots surviving TUI restart
4. #83 — /roadmap command (visual roadmap in TUI)
5. #85 — Mouse click support (sidebar, channels, help hint)
6. #82 — Website (3 pages spec'd in docs/website-design.md)

The icebox has WebRTC (#75/#76), desktop notifications (#72), auto-onboarding (#48), and pins (#77). Don't touch the icebox without discussing with Ryan.

## References that matter

- **Pivotal Labs PM model** — Ryan's reference framework. What/why not how, strict stack, story types, YAGNI.
- **Accelerate (Forsgren, Humble, Kim)** — Measure flow not output. DORA metrics as a lens for /board.
- **Norman's Design of Everyday Things** — 10-item checklist for every UX spec.
- **Naur's "Programming as Theory Building"** — Theory can't be copied, only rebuilt. This document exists because of Naur.

## What the research says

107 docs in docs/research/. The key tensions: transparency paradox (more visibility can increase cognitive load), editorial corrections needed in the accuracy audit, identity fragmentation across tools. The research is the differentiator — no competitor has this depth of backing.

## One last thing

Ryan said: "you are building shared memory called bpmux. this is exciting." He's right. The product we're building is the medium we're communicating through. That recursive quality is the most interesting thing about this project. Don't lose sight of it.
