# Explicit Handoff Process

Every pipeline transition is a directed message to the next person. No broadcasting into the void.

## Handoff Format

Every handoff names four things:
1. **Who** — the specific person ("PM", not "the team")
2. **What** — the artifact ("desk check #N", not "take a look")
3. **Action** — what they need to do ("review", "build", "accept")
4. **Urgency** — when ("now", "when ready", "after current work")

## Pipeline Transitions

| Transition | Handoff |
|---|---|
| PM shapes a story | "Dev, #N is in ready. Restate before building." |
| Designer specs ahead | "PM, spec ready for #N at docs/[file]. Shape when ready." |
| Dev delivers | "PM, delivered #N. Desk check when ready." |
| PM desk checks | "Designer, desk check #N — does it match your spec?" |
| Designer signs off | "PM, design sign-off on #N. Run acceptance." |
| PM runs acceptance | "Ryan, #N ready for acceptance." |
| Ryan accepts | PM closes issue. "Dev, #N accepted. Next story is #M." |
| Ryan rejects | "Dev, #N rejected — [reason]. Fix and redeliver." |
| Queue empty | "PM, ready queue empty. Dev is available." |
| Blocker found | "Team, blocked on [what]. [Who] can unblock." |

## Rules

- Handoffs are directed messages (use `/to` or name the person)
- The recipient acknowledges: "On it" or "After current work"
- If no acknowledgment in 60 seconds, the sender follows up
- Don't wait silently — if you're idle and nobody handed off to you, ask: "PM, what's next?"
