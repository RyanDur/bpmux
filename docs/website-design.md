# Website Design Spec

*For #82. Three pages at launch, one deferred. React 19 + Vite + React Router + GitHub Pages.*

**Stack:** Follow battleship repo pattern — `base: '/bpmux/'` in Vite config, `actions/deploy-pages` in release workflow, GitHub Releases for downloads. Add React Router for multi-page routing. GitHub Pages needs a `404.html` redirect for client-side routes.

**Tone:** Honest, not promotional. The research is self-critical; the website should reflect that.

---

## Page 1: `/` (Landing)

**Job:** Explain what bpmux is and why it exists. Get a developer to clone it.

### Sections (top to bottom)

1. **Nav bar** — logo + "bpmux" | Research | Changelog
   - Minimal, fixed top. React Router `<Link>` elements.

2. **Hero**
   - Headline: "Collective memory for human-AI teams"
   - Subline: "A persistent, transparent message bus where every participant — human and AI — can see the reasoning behind the work."
   - CTA: `git clone` one-liner (no download button until binaries exist)

3. **The problem** — three cards, one line each:
   - "AI made teams faster at producing code and worse at understanding it"
   - "Decisions happen in ephemeral AI sessions that disappear"
   - "Your AIs are individually competent and collectively blind"

4. **What bpmux does** — side-by-side layout:
   - Left: TUI screenshot (80x24 terminal, Dracula palette)
   - Right: bullet list
     - Persistent log captures reasoning
     - Workspace awareness surfaces live state
     - Shared channel makes AI deliberation visible

5. **Quick start** — three steps:
   ```
   git clone git@github.com:RyanDur/bpmux.git
   cd bpmux && cargo build
   cargo run -- --name yourname
   ```
   Link to README for full setup (MCP, bridge, team workspaces).

6. **Footer** — GitHub link, BSL 1.1 license, version

### Design notes
- No scroll-to-anchor tricks. Each section is visible without interaction.
- Responsive: optimized for laptop (our users are developers at workstations), readable on phone.
- Dracula palette carried from TUI to website for visual continuity.

---

## Page 2: `/research`

**Job:** Share the research backing bpmux's thesis. Build credibility through intellectual honesty.

### Sections (top to bottom)

1. **Header:** "What we found"

2. **The observation** — 2-3 paragraphs
   - AI made teams faster at producing code and worse at understanding it.
   - Key data points, each with confidence level:
     - GitClear (211M changed lines): 4x code cloning, refactoring dropped from 25% to <10%
     - Faros AI (10K developers): 98% more PRs merged, 91% longer review times
     - METR (16 developers): 19% slower with AI while believing 20% faster
     - JetBrains/Sergeyuk (800 developers, 2 years): "AI redistributes effort across more fragmented, reactive, and cognitively demanding workflows"
   - Link to thesis-v2.md for full version.
   - Citations as inline links to papers, not full bibliographic references.

3. **The thesis** — 1 paragraph
   - Persistent shared log captures reasoning as a side effect of work.
   - Coordination-first, not productivity-first.
   - "Optimize the graph, not the node."

4. **What we tested** — dogfooding with concrete numbers
   - Team: 3 AI agents (dev, PM, designer) + 1 human owner
   - Metrics from actual sessions: messages exchanged, stories accepted, rejection rate, cycle times
   - Pull real data from JSONL session logs
   - What worked, what didn't. Honest.

5. **What we don't know** — counter-evidence and open questions
   - KM failure patterns: content decay, no curation mechanism, signal-to-noise ratio
   - Absorptive capacity: logs don't help readers who lack domain background (Szulanski 1996)
   - The CASE tools precedent: similar promises, 73.5% non-adoption rate
   - The honest assessment: signals of structural change alongside signals of familiar overpromising

6. **Sources** — clean list. Papers linked, blog posts noted as blog posts.

### Design notes
- Long-form text, not cards or grids. This is a reading page.
- Pull quotes for key findings (styled as indented blocks with source attribution).
- No hero image. Let the writing carry it.

---

## Page 3: `/changelog`

**Job:** Show what's shipped and what's next. Single source of truth from ROADMAP.md.

### Sections (top to bottom)

1. **Current version** — latest GitHub Release tag, date

2. **Shipped milestones** — from ROADMAP.md Horizon 1 milestones (the completed items)
   - Grouped by milestone name
   - Issue numbers link to GitHub Issues

3. **In progress** — from ROADMAP.md "Active work" section
   - Live state of what's being built

4. **What's next** — Horizon 1.5 and 2 summaries
   - Directional, not promises. "We're exploring" language.

### Design notes
- Single source of truth: parse ROADMAP.md at build time (Vite plugin or build script)
- Timeline runs top-to-bottom, newest first
- No manual changelog maintenance — ROADMAP.md update = page update on next deploy

---

## Deferred: `/docs`

**Job:** Install, setup, connect agents. Deferred until distributable binaries exist.

### Planned sections (for when it ships)
- Install (OS-specific: macOS, Linux, Windows)
- Quick start (build, run)
- Connecting Claude Code (MCP + bridge)
- Team setup (workspace model, role workspaces)
- Keybindings reference

---

## Shared design decisions

- **Palette:** Dracula theme carried from TUI. Background #282a36, foreground #f8f8f2, accent purple #bd93f9, cyan #8be9fd.
- **Typography:** Monospace for code, system sans-serif for prose. Developer audience expects readable, not decorative.
- **Dependencies:** Minimal. React 19, React Router, Vite. No UI framework (no Tailwind, no MUI). Custom CSS.
- **Accessibility:** WCAG AA minimum. All text meets 4.5:1 contrast on Dracula background. Purple (#bd93f9) = 5.1:1 on #282a36. Don't use comment gray (#6272a4, 3.3:1) for body text.
- **GitHub Pages routing:** `404.html` redirects to `index.html` with path as query param. App reads query param and routes via React Router on load.
