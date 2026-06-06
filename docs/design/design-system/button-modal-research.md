# Design-system research — buttons + modals (Ryan-provided)

**Provenance:** Ryan-provided in the bpmux channel 2026-06-06 (re-paste of his
Grus-close directive + research). Captured here so the source material has a
**discoverable, durable home** — not only the chat log / briefing. The recurring
pain it cures (Ryan, 2026-06-05/06): after a clear we mis-ground because the plan
+ its research lived in directives + a lossy briefing, in no durable entity. This
doc is the firsthand substrate, preserved independent of any one iteration entity.

**Status:** RESEARCH CAPTURE only — for the **design-system EPIC**. Per the
corrected plan (2026-06-06): the design-system is to be DISCUSSED during the current
channels-independence / Pyxis #1521 iteration and PLANNED as a FUTURE iteration. It
is NOT the current iteration's theme and is NOT a vehicle for the rejected stories.
The *synthesis* — laying this grammar onto the IntelliJ-Darcula visual base,
per-component designs, the unified status/signaling language, mockups — is
mockup-first + Ryan-reviewed, done when the design-system iteration is planned +
cleared. Do not treat the numbers below as shipped tokens until synthesized.

---

## Ryan's directive (the synthesis brief)

> synthesize the design systems against our aesthetics — a design for each
> component + the overall look. IntelliJ **Darcula is the base**.

Read: the systems below are the **structural grammar** (spacing, proportion,
hierarchy, sizing). Darcula is the **visual base** (palette, density, dark-IDE
feel). Lay the grammar onto Darcula — adopt the *structure*, NOT the source
systems' colors.

---

## Buttons

Standardize on an **8px (or 4px) spacing grid** for harmonious proportions.

- **Padding** — proportional to font size; baseline rule **horizontal = 2× vertical**
  (e.g. 16px horizontal / 8px vertical). *(This 2:1 ratio is already seeded into the
  #1518 reject-modal spec — `--space-8`/`--space-16` — forward-compatible with this
  grammar; note #1518 itself ships on CURRENT tokens within Pyxis #1521, not on this
  future design-system.)*
- **Dimensions** — consistent heights across sizes: **Small 32px · Medium 40px ·
  Large 48px**. Minimum **touch target 44×44px** (WCAG 2.2 target-size minimum is
  24×24px; 44px is the comfortable target).
- **Label & icons** — concise text (1–2 words); **8px** between icon and label.
- **Groups** — separate buttons by **8px** to read as a connected group.

## Modals

### Sizing & structure
Scale responsively; align to the baseline grid. Use **percentage / breakpoint
columns**, not fixed px:

- **Extra Small** — 24%–48% of screen width
- **Medium (default)** — 48%–60%
- **Large** — 60%–80%
- **Content margins** — strict **16px or 24px** between outer edge and inner text,
  all sides.

### Padding & spacing (hierarchy)
- **Header / title** — larger text (~**20px**) with **24–32px** padding separating
  title from body copy.
- **Footer / actions** — **16–24px** top/bottom padding separating actions from body.
- **Action layout** — primary/secondary **stack vertically full-width on mobile**
  (easier touch targets); **align bottom-right on desktop**.

---

## Sources (Ryan-provided, 14 refs)

1. Carbon — modal style — https://carbondesignsystem.com/components/modal/style/
2. VA — modal design decisions — https://design.va.gov/components/modal/design-decisions/
3. GitLab — modal — https://design.gitlab.com/components/modal
4. VA — button — https://design.va.gov/components/button/
5. EightShapes — size in design systems — https://eightshapes.com/articles/size-in-design-systems/
6. ServiceNow Horizon — spacing usage — https://horizon.servicenow.com/workspace/foundations/spacing/spacing-usage
7. Foundational design system — 7 core elements — https://medium.com/@davefelix247/foundational-design-system-the-7-core-elements-d2150aa254fd
8. UXLab — spacing guide — https://www.uxlab.academy/blogs/the-ultimate-spacing-guide-for-ui-designers
9. Uxcel — anatomy III — https://uxcel.com/lessons/anatomy-iii-298
10. BTNG Studio — design system ultimate guide — https://www.btng.studio/articles/design-system-ultimate-guide/
11. Carbon — button usage — https://carbondesignsystem.com/components/button/usage/
12. Designing buttons — best practices — https://medium.com/design-bootcamp/best-practices-for-designing-buttons-a45d721d77d9
13. UX Mag — designing the perfect button — https://uxmag.com/articles/designing-the-perfect-button
14. Carbon — modal style (no-cache) — https://carbondesignsystem.com/components/modal/style/?no-cache=1

*(Caveat noted by Ryan: this research was AI-assembled — verify against the primary
source docs during synthesis, esp. the Carbon / GitLab / VA component pages.)*

---

## Synthesis scope (FUTURE design-system iteration — NOT yet done)

When the design-system epic is planned + cleared, the mockup-first synthesis expands this into:
- the **Darcula visual base** (palette / density / dark-IDE feel) the grammar lands on;
- a **unified status / signaling language** — ONE grammar across board-sync / CI /
  online (Ryan's coherence question — today three signals = three visual languages);
- **per-component designs** (button, modal, list / `<SortableList>`, iteration-row) + the overall look.

**The rejected stories are NOT consumers of this future system.** #1518 / #1524 /
#1517 live in the CURRENT iteration (Pyxis #1521) and ship on CURRENT tokens. Their
visual surfaces (modal, SortableList, iteration-row) are LIKELY future consumers once
the system lands; #1518's 2:1 button is a forward-compatible seed only.

Each synthesized component is mockup-first + Ryan-reviewed before build.
