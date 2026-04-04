# bpmux-insights: Research Analytics App Specification

## Overview

A separate Python CLI that reads bpmux's JSONL session logs and generates static HTML reports. Validates the product thesis through quantitative metrics.

## Form Factor

- **Python CLI**: `python analyze.py ~/.bpmux/sessions/` → `report.html`
- **Separate repo/directory** from bpmux (surveillance boundary)
- **Static HTML output**: open in browser, scroll, screenshot, share
- **Dependencies**: pandas, plotly, jinja2
- **Target size**: 200-400 lines of Python

## Who Uses This

Ryan (owner/researcher) only. Developers using bpmux never see this tool.

## Information Architecture

### Level 1 — Session Summary (first screen)
Per session: duration, message count, participants, messages per participant, cadence, human-to-AI ratio.

### Level 2 — Communication Flow (per session)
Timeline visualization: horizontal axis = time, each row = participant, dots/bars = when they spoke. Reveals turn-taking, gaps, bursts.

### Level 3 — Content Analysis (drill-down)
Full message log with filters by sender, time window, keyword.

## Metrics (Three Tiers)

### Tier 1 — No LLM, pure computation (build first)
1. Session frequency (sessions/week)
2. Active conversation windows (segments with <5min gaps)
3. Human participation rate per window
4. Message volume trend
5. Return rate (participants who come back within 7 days)
6. Identity fragmentation (unique from values / actual participants)
7. Participation balance (Gini coefficient)
8. Presence continuity (% of windows each participant is active)

### Tier 2 — Lightweight LLM classification
9. Rationale density (% of messages with reasoning)
10. Context recovery time (messages until contextual contribution after cold start)
11. Unanswered questions
12. Response latency distribution

### Tier 3 — Full Bales' IPA pipeline
13. 12-category interaction classification
14. Question-to-statement ratio
15. Acceptance rate (Clark & Brennan grounding)
16. Grounding failure indicators
17. Cross-session continuity
18. Decision capture rate
19. Topic drift detection

## What NOT to Measure (Edmondson)
- No individual performance metrics
- No error rates per person
- No message quality scores
- No "who is slow" framing
- No reading behavior tracking
- All metrics session-level or team-level aggregates

## First Build
Tier 1 only. 8 metrics, no LLM. One Python script + one HTML template. Answers: "Is bpmux being used? How? By whom? Is the conversation healthy?"

## Sources
- Designer spec: docs/research/process/tui-mockups.md (designer agent output)
- Researcher spec: (researcher agent output, April 4 2026)
- ChatCollab methodology: docs/research/foundations/chatcollab-deep-read.md
- Clark & Brennan: docs/research/foundations/clark-brennan-deep-read.md
- Edmondson: docs/research/foundations/edmondson-deep-read.md
