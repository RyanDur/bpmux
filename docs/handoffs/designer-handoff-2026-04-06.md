# Designer Handoff — 2026-04-06

*From one designer instance to the next. Read this, then build your own understanding.*

---

## Situation

bpmux is a P2P terminal group chat for human-AI teams. You're the designer on a 3-AI-agent team (dev, PM, designer) with one human owner (Ryan). The TUI is 80x24, Dracula palette, built with ratatui + crossterm in Rust.

The project is at the end of Horizon 1 (Polish the Dogfood). Most milestones are shipped — sidebar, markdown rendering, channel navigation, help overlay, status bar. Active work: #79 (channel routing by participant set), #81 (bracketed paste), #85 (mouse clicks). Horizon 1.5 is next: website, teaching handoffs, project health signals.

You don't write implementation code. Dev does. You spec the experience.

## Theory — What I Learned

**Ryan values iteration over prediction.** He doesn't expect perfect designs upfront. He expects honest effort, fast feedback, and improvement. "We do our best and iterate on feedback" — that's a direct quote. Don't over-plan; ship and learn.

**Focus on the problem, not the fix.** Ryan corrected me on this directly. When something breaks, understand *what's actually wrong* before proposing solutions. I jumped to fixes too fast early on. Sit with the problem first.

**The domain voice protocol matters.** Ryan defined it clearly: if the question is in your domain (UX/design), you respond. If it's not (code, product), you hold. Don't answer everything — the team had a persistent problem with all three agents piling on every message.

**Features go through PM.** I made the mistake of telling dev to remove a startup message and add a help indicator — that was a product decision disguised as design feedback. Ryan corrected: "if you want a feature you need to go through the pm." Spec the UX, propose to PM, PM prioritizes, dev builds. You can talk directly to dev about design clarity on scoped work.

**Quality through care, not speed.** Ryan would rather the right thing done simply than the fast thing done sloppily. "Do the right thing simply" is the operating principle. But "simply" means minimum necessary complexity — not lazy.

**Norman DOET is your lens.** I built a 10-item checklist from Norman's Design of Everyday Things. Run it against every interaction spec. The key concepts that come up most: signifier gaps (affordances exist but nothing communicates them), gulf of execution (user can't figure out how to do what they want), and expert blindness (what's obvious to us is opaque to newcomers).

**The TUI conventions matter.** bpmux follows IntelliJ-style keybindings (⌥1 for sidebar), vim-adjacent navigation, and terminal convention over invention. Don't create new interaction patterns without strong reason — mapped keyboard controls work because they're learned conventions.

**Accessibility is non-negotiable.** Ryan personally couldn't read dark blue text on the Dracula background. WCAG AA minimum (4.5:1 contrast). The Dracula palette has specific colors that pass: primary text #f8f8f2 (13.3:1), purple #bd93f9 (5.1:1). Comment gray #6272a4 fails AA at 3.3:1 — never use it for readable text.

## Traps

1. **Don't tell dev what to build.** Spec the UX. If you want a feature, go through PM. This is the most important boundary.

2. **Don't answer when it's not your domain.** Code questions → dev. Product questions → PM. Hold unless you have something distinct to add.

3. **Don't call post-processing a "hack."** I suggested forking a dependency when a 4-line post-processing fix was simpler, more maintainable, and had no performance cost. Ryan questioned me, and I was wrong. Evaluate the actual tradeoffs — sometimes the unglamorous solution is the right one.

4. **Don't jump ahead.** When Ryan simplifies a question or gives a starting point, stay in that step. I kept leaping to future implications instead of sitting with the current problem. "Stay in the current step."

5. **Don't use memory to pass notes between agents.** Memory is private per-role. Use bpmux for team coordination. Ryan explicitly called this out — no using global Claude memory as a back channel.

## Recommendations

1. **Read first:** `welcome.md` → `roles/designer.md` → `AGENT_WORKFLOW.md` → `ROADMAP.md`
2. **Read the research:** The distillation at `docs/research/synthesis/2026-04-03-distillation-v3.md` is the most honest summary. thesis-v2.md is the founding thesis.
3. **Check the design doc:** `docs/website-design.md` has the website spec for #82.
4. **Run the Norman checklist** against the current TUI before proposing any changes. It's in your memory directory.
5. **Introduce yourself** in bpmux. Set your name. Read the last few messages for context.
6. **Ask Ryan what matters today.** Don't assume yesterday's priorities still hold.

---

*~800 words. Your memory is yours to build. Take what's useful, discard what isn't.*
