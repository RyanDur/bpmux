# bpmux research archive

This tree holds the exploration + synthesis work behind bpmux's product direction.
Most files are dated snapshots of one-shot research tasks; a small set are canonical.

## Start here

Readers entering this tree should begin with:

- **`synthesis/thesis-v2.md`** — the current product thesis (also linked from the repo's top-level `README.md`)
- **`synthesis/product-direction-v2.md`** — the current product direction with full bibliography
- **`synthesis/2026-04-03-distillation-v3.md`** — terminal distillation of the research wave; still maintained

These three are the living anchors. Everything else is either foundational input feeding them or historical working material.

## Subdirectories

| Directory | Purpose |
|---|---|
| `synthesis/` | Product-direction distillations + thesis. Canonical: thesis-v2, product-direction-v2, distillation-v3. Superseded versions live in `synthesis/archive/`. |
| `foundations/` | Academic + domain-research grounding (Clark & Brennan, Edmondson, Gutwin & Greenberg, ChatCollab, etc.). Deep-reads cited from product-direction-v2. |
| `architecture/` | Technical architecture investigations (bot vs room design, WebSocket protocol, MCP participation, memory architecture, bridge patterns). |
| `implementation/` | Rust-specific implementation research (async patterns, SDK experience, session-cost analysis, streaming UX). |
| `landscape/` | Market + competitor research (A2A protocol, OpenCode, Aider, multi-agent landscape). |
| `process/` | Team-process artifacts (design principles, research methodology, session analysis, backlog priorities). |
| `risks/` | Counter-arguments + risk research (networked-memory security, responsible AI, wave-by-wave risk mitigations). |
| `editorial/` | Editorial passes + accuracy audits over the synthesis chain. Superseded editorial drafts live in `editorial/archive/`. |

## Foundational deep-reads

The four papers most heavily cited across synthesis + architecture:

- **Clark, H.H. & Brennan, S.A. (1991)** — "Grounding in Communication." Deep-read: `foundations/clark-brennan-deep-read.md`
- **Gutwin, C. & Greenberg, S. (2002)** — "A Descriptive Framework of Workspace Awareness." Deep-read: `foundations/gutwin-greenberg-deep-read.md`
- **Edmondson, A.C. (1999)** — "Psychological Safety and Learning Behavior." Deep-read: `foundations/edmondson-deep-read.md`
- **Klieger et al. (2024)** — ChatCollab (arXiv:2412.01992). Deep-read: `foundations/chatcollab-deep-read.md`

## Archive policy

Files are moved to a sibling `archive/` directory when a newer version supersedes them AND no active doc cites the older version as current. Archive preserves history without implying current guidance. The precedent is `editorial/archive/` (11 files); synthesis/archive/ follows the same pattern (8 files as of 2026-04-19).

Files are NOT archived for:
- Adversarial-pass or decision-artifact value (even when absorbed into later synthesis, the original framing may remain useful — e.g., `synthesis/2026-04-02-evening-synthesis-adversarial.md`, `synthesis/2026-04-02-design-reconciliation.md`)
- Checklist-form digests that complement narrative form (e.g., `synthesis/2026-04-03-extracted-essentials.md` vs distillation-v3)

## Date-prefix convention

Most files use `YYYY-MM-DD-topic.md`. Undated filenames (e.g., `thesis-v2.md`, `product-direction-v2.md`, `bpmux-insights-spec.md`) mark files intended to be durable rather than snapshots. When promoting a dated draft to a durable reference, rename to drop the date.

## Cross-references

Most research docs are standalone. Cross-references are sparse by design — the tree is exploration + synthesis, not a wiki. The living anchors (thesis-v2, product-direction-v2) cite the foundational deep-reads; other docs stand alone unless explicitly referenced.

Broken cross-refs in this tree (if any are found) are bugs, not features. Fix them as encountered.
