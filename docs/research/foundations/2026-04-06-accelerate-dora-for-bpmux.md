# Accelerate & DORA Metrics Applied to bpmux

*Designer research — 2026-04-06. Directed by Ryan.*

## Source

Forsgren, Humble, Kim. *Accelerate: The Science of Lean Software and DevOps* (2018). Based on 23,000+ survey respondents across four years of State of DevOps Reports. Statistical method: structural equation modeling, cluster analysis.

## The Core Finding

Speed and stability are NOT tradeoffs. High-performing teams deploy 46x more frequently, with 440x faster lead times, 170x faster recovery, and 5x lower failure rates. They are better at both simultaneously. This overturns the conventional "move fast and break things vs. move carefully" dichotomy.

## The Four DORA Metrics

| Metric | What It Measures | High Performers | Low Performers |
|---|---|---|---|
| Deployment Frequency | How often you ship | Multiple times/day | Once/month to once/6 months |
| Lead Time for Changes | Commit to production | < 1 hour | 1-6 months |
| Change Failure Rate | Deployments causing issues | 0-15% | 31-45% |
| Mean Time to Recover | Time to restore service | < 1 hour | 1 week to 1 month |

## Mapping to bpmux's Team

Our team is 1 human + 3 AI agents. The metrics translate:

- **Deployment Frequency** → Stories accepted per session. Tonight: 11 accepted. That's our tempo signal.
- **Lead Time** → Time from story start to acceptance. Sub-intervals matter: work time vs. wait-for-acceptance time.
- **Change Failure Rate** → Stories rejected at acceptance. Tonight: 2 rejected out of 13 presented (~15%). That's already at the high-performer boundary.
- **MTTR** → Time from rejection to re-acceptance. Tonight's rejections were fixed within minutes.

## The 24 Capabilities — What Matters for Us

### High leverage at our scale

1. **Small batch sizes** — Stories completable in hours, not days. Strongest lever for both speed and stability.
2. **Trunk-based development** — We do this. Short branches, frequent merges.
3. **Test automation** — Every story includes tests. `cargo test` is our CI.
4. **WIP limits** — Ryan is the bottleneck for acceptance. WIP should reflect his review capacity, not agent throughput.
5. **Visual management** — Make work visible. The board should surface flow, not just status.
6. **Loosely coupled architecture** — Enables parallel agent work without merge conflicts.
7. **Westrum generative culture** — Blame-free inquiry on rejection. Ryan already does this: "focus on the problem, not the fix."
8. **Documentation quality** — For AI agents, written context is the ONLY context. Role contracts, stories, and CLAUDE.md are load-bearing infrastructure.

### Less relevant now (relevant later)

- Disaster recovery testing (no production service yet)
- Shift left on security (not the current bottleneck)
- Monitoring/observability (relevant when bpmux is deployed)

## What the Board Should Surface

Based on Accelerate, the signals that predict health:

1. **Throughput** — Stories accepted per week (deployment frequency analog)
2. **Cycle time** — Story start to acceptance, broken into work vs. wait
3. **Rejection rate** — Rejected / total delivered (change failure rate analog)
4. **WIP count** — Stories in progress now vs. human review capacity
5. **Batch size** — Average story completion time (smaller = better)
6. **Rework ratio** — Delivery cycles before acceptance (1 = clean, 2+ = rework)

## The Human-AI Team Problem

Accelerate assumes human teams. For our team:

**Burnout risk is concentrated.** Ryan bears 100% of acceptance load. If agents produce faster than he can review, the constraint isn't throughput — it's review capacity. WIP limits exist to protect the human.

**Psychological safety applies asymmetrically.** Agents don't feel fear, but Ryan needs to feel comfortable slowing agents down, rejecting work, and saying "I don't understand." The system should make this frictionless.

**Culture is defined by the human, enacted by agents.** How Ryan responds to rejected stories IS the culture. Inquiry ("what was unclear?") vs. blame ("you got it wrong") sets the norm. Ryan's consistent "focus on the problem" framing is textbook generative culture.

**The feedback loop is compressed.** Human teams improve over months. We can change agent behavior instantly by updating role contracts or CLAUDE.md. Our improvement cycle is minutes, not months. Retros and process changes can be more frequent and precise.

**Documentation is infrastructure.** In Accelerate, documentation quality is one of 24 capabilities. For us, it's arguably the most critical — it's the only way institutional knowledge persists across agent sessions. Memory files, role contracts, and story definitions are not overhead. They're the mechanism that makes everything else work.

## Connection to Existing bpmux Research

Our measurement framework (`docs/research/architecture/2026-04-02-measurement.md`) already defines 9 metrics. Accelerate adds the interpretive frame:

| Our Metric | Accelerate Analog | What It Tells Us |
|---|---|---|
| Context Recovery Time | (no direct analog) | Unique to AI teams — measures memory effectiveness |
| Session Continuity | Documentation Quality | Whether written context actually bridges sessions |
| Human Participation Rate | (related to WIP/capacity) | Is the human engaged or overwhelmed? |
| Decision Quality | Change Failure Rate | Are we making good decisions? |

The metrics we're missing that Accelerate would add: cycle time, WIP count, rework ratio.

## Criticisms of the Framework

- All data is self-reported survey data, not objective measurement
- Cross-sectional design can't prove causation (despite SEM claims)
- Aggregates across industries/sizes — what works at 10,000 people may not apply to 4
- Metrics can be gamed (deploy a no-op to inflate frequency)
- Predates AI-assisted development entirely

## Recommendation

Surface flow metrics on the board: throughput, cycle time, rejection rate, WIP. These are the signals Accelerate says matter. Don't add them as a dashboard — integrate them into the existing /board command so Ryan can glance and know whether the project is healthy. Start simple: accepted count, rejected count, active WIP. Add cycle time when we have enough data points.
