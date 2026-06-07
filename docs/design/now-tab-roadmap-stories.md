# Now-tab → roadmap — story specs (designer)

Designer-authored UX specs for the 2 thin slices Ryan approved 2026-06-06 (now-tab empty-state
→ roadmap; gap-filler v1 + on-demand 🗺). Design-of-record: `now-tab-empty-state-roadmap-mockup.html`.
PM creates the Feature-entities + adds to the next iteration; dev-feedback adjusts at IPM (per Ryan).
Acceptor: **owner** (Ryan SEES the roadmap) → owner-walk at acceptance.

> **Spec discipline:** each story leads with the `observation_falsifier` ("I would reject if I
> observed X") — the failing-test-shape as AC. Implementation (reuse the `/roadmap` render) lives in
> *What*, not the AC. The AC pins observable behavior, not internals.

---

## Story 1 — Empty now-tab falls back to the roadmap (v1, gap-filler)

**User-job:** When no iteration is active, I see the roadmap in the now-tab instead of an empty
board — so at the close→open gap I know what's next, right when we're deciding the next iteration.

**What:** When there is no current iteration, the now-tab renders the roadmap (reuse the existing
`/roadmap` / `view_roadmap` render — see dependency) in place of the empty board. When an iteration
opens, the now-tab returns to the board automatically. No new navigation — it's a state-driven
fallback on the surface the user already lands on.

**Why:** A dead empty-state is a missed moment (Nielsen: empty-states orient + guide, never
dead-end). The now-tab answers "what's the team doing?" — and when nothing's active, the coherent
answer is "what's NEXT" = the roadmap. The close→open gap is exactly when that's most useful.

**AC / observation-falsifier — I would reject if I observed:**
1. An empty / blank / dead now-tab (no board AND no roadmap) when there is no active iteration.
2. The roadmap renders but with STALE or WRONG content — not the current `ROADMAP.md` (see the #6
   dependency). The falsifier pins the RIGHT content (e.g. the current pipeline's next iteration is
   present + correct), not merely "something roadmap-shaped appears."
3. The now-tab stays stuck on the roadmap after an iteration opens (the fallback must be reactive to
   iteration-state, not sticky) — opening an iteration must flip it back to the board.

**Observable (the walk):** close the current iteration → the now-tab shows the roadmap (real
pipeline + horizons from `ROADMAP.md`) → open the next iteration → the now-tab shows that iteration's
board.

**Dependency to ground at pickup:** the `/roadmap` render ties to **bug #6** (verify `view_roadmap`
reads `ROADMAP.md`, not a stale/wrong source). If #6 is live, falsifier #2 fails until it's fixed —
so #6 is either in-scope here or a hard predecessor. Ground this firsthand when speccing the AC.

---

## Story 2 — On-demand 🗺 roadmap affordance (reach the roadmap while active)

**User-job:** I can pull up the roadmap on-demand while an iteration IS active — so I can check
what's next without waiting for the gap.

**What:** A 🗺 affordance (button / command) that surfaces the same roadmap render while an
iteration is active; dismissing it returns to the board. Same content + source as Story 1 (one
roadmap render, two triggers: the empty-state fallback and this intentional reach).

**Why:** The gap-filler only shows the roadmap when nothing's active; sometimes the owner wants
"what's next" mid-iteration. This adds the intentional reach without duplicating the render.

**AC / observation-falsifier — I would reject if I observed:**
1. No way to reach the roadmap while an iteration is active (the affordance is absent or dead).
2. The on-demand roadmap shows different / staler content than the Story-1 fallback — they must
   share one source (`ROADMAP.md`), so the two surfaces never disagree.
3. Invoking it loses the board / can't return — it must be a non-destructive overlay/view that
   dismisses back to the active board.

**Observable (the walk):** with an iteration active, invoke the 🗺 affordance → the roadmap shows
(same content as the fallback) → dismiss → back to the active board.

**Sequencing:** Story 2 builds on Story 1's render. Smallest-first: Story 1 leads; Story 2 follows.
