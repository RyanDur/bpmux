# Backlog view — the PM's needs (discovery)

*Designer discovery with the PM, 2026-06-30. The PM is the primary
user of the backlog surface; Ryan (owner) wants a window into it to
plan with the PM. Captured firsthand from the PM's lived workflow, then
grounded against the `bpmux` substrate. This is the input to the
design Ryan + designer synthesize together — not the design itself.*

## ⚠ Premise-correction (2026-06-30)

The board/backlog split **largely already exists** in the shipped UI:
Board → `[Now | Plan | Done]` tabs, where **Now** = the current-iteration
projection and **Plan** = all epics + iterations + icebox = the backlog
ledger. This is the designer's own settled design. The jobs + pains
below remain **valid** (PM's lived workflow) — but read them as needs
the *existing* surfaces should serve, not a from-scratch build.

**Design-of-record = the reorganization** (Ryan-directed 2026-06-30):
`board-plan-reorganization-mockup.html` — Board = the current iteration;
Plan (was Roadmap) → sub-views `[Roadmap | Backlog]`; Done tucked.

Two considerations carried from the retired `placement-options.md`
(which was a phantom — premised on a mixed board that's already tabbed):
- **Authoring is a *visibility* need, not a GUI-canvas** — PM authors
  via CLI; the surface serves authoring by showing the ledger during
  the dup-check (PM's catch). A GUI authoring form is a separate
  owner-vision bet, not assumed.
- **Dependency-structure / the PLAN-gate has no home in the IA yet** —
  the views show *order*, not which stories can't co-exist in one
  iteration. A real later-slice gap; flagged so it isn't dropped.

## PM re-ground (firsthand, 2026-06-30) — most pains are already VIEW-served

PM re-grounded the deeper pains against the real components (`StoryRow`
· `epicSummary` · `Iteration` · `BoardGroup`). The discovery's
"nothing exists" was an overstatement — **most pains are already served
by the existing GUI:**
- *Ordering "blind"* — REFUTED for the view: `StoryRow` renders
  position/total + drag-grip + keyboard reorder (`SortableList`). The
  ordered list is visible + directly manipulable.
- *Iteration-compose "no single view"* — largely refuted: `Iteration`
  shows Active+Planned (drag) + Closed + Unscheduled with members.
- *Epic slice-coverage "guesswork"* — largely refuted:
  `epicSummary.breakdownFor` computes the child-state breakdown +
  progress.
- *Icebox "buried"* — refuted (it's a group under Plan).
- Some triage signals exist already (blocked badge, rework glyph).

**Genuinely surviving VIEW-gaps (narrow — by absence in `StoryRow`):**
1. age / staleness / last-touched · 2. dup / overlap signal ·
3. dependency-structure (the PLAN-gate). *(+ acceptor-label = dev's CLI
fix, already routed.)*

**THE REFRAME (synthesis question for Ryan):** the discovery's pains
were largely about PM's *CLI/MCP tooling* (where the agent actually
authors/triages/orders), but this VIEW is the Electron renderer =
*Ryan's window* into the backlog. The existing view already solves most
VIEW-equivalents of those CLI pains. So the residual is (a)
**glance-quality** of the existing view (the original board-view
problem — "Ryan asks how's the backlog and waits for a dump" is a
glance gap, not a does-it-exist gap) + (b) the 3 signal-overlays. Open
for Ryan: is the deeper work his **VIEW** (glance + signals), PM's
**CLI tooling**, or **both**? — that conflation inflated the gap.

**Net shape:** the reorg re-chrome (Ryan-directed) + maybe 2-3
signal-overlay slices + a glance-quality pass — NOT a big net-new view.
*(The original discovery below is preserved as authored; read it through
these corrections.)*

## North-star (the through-line)

Every backlog job today is **invisible-until-manually-read**. Dupes,
staleness, story age, ready-queue order, iteration composition, Epic
slice-coverage, dependency structure — none of it is ever *shown*; the
PM reconstructs it by hand from CLI reads every time. And the icebox /
ledger is buried in the *operational* board it doesn't belong in.

> The backlog view's job is to **make backlog state glanceable** — the
> same "the view should BE the answer" thesis the [board-view problem
> doc](index.md) named, now applied to the ledger.

Substrate-grounded: the `bpmux` surface is all CLI *reads*
(`backlog`/`board`/`ready`/`story`) with no persistent view; the icebox
is one collapsed column inside the mixed operational board
(`board/types.ts` `GROUP_ORDER`: ready · in-progress · desk-check ·
delivered · accepted · retired · icebox — all one panel). No backlog
view exists in the renderer today.

## The four jobs

### 1. Author a story → icebox
**Today:** `bpmux create` (title · user-job · observation-falsifier ·
Gherkin ACs · acceptor · parent-epic · acceptance-test-path), gated
through 3 skills (classify-story-type · pre-filing-audit ·
pre-filing-view-backlog). Lands in icebox.

**Pains**
- Authoring **blind to the existing backlog** — the dup/overlap check
  is a separate `view_backlog` run held in the head, a context-switch
  not a glance.
- The **field-shape is scattered** (story-shape doc + 3 skills) —
  nothing walks the fields at the point of authoring; assembled from
  memory each time.
- After filing, **can't see where it landed** relative to its siblings.
- *(Filing-ergonomics bug, separable):* `create_story` leaves the
  acceptor **label** unapplied — a separate step that's easy to miss,
  files the story unlabeled. → flag to dev/PM as a fix, not view-scope.

**Design implication:** author *with the backlog visible* (dup-check
becomes a glance); shape present at point-of-authoring; show where the
new story lands among siblings.

### 2. Triage the icebox  *(the worst surface today)*
**Today:** `view_backlog` + `board` + `story`, manual eyeball;
supersede/retire.

**Pains**
- The **icebox is effectively invisible** — crammed into the mixed
  board, never seen as its own clean "captured-but-not-ready" list.
  *(The split fixes exactly this.)*
- **Dupe/staleness detection is all manual**, story-by-story. Nothing
  surfaces overlapping ACs or "untouched N days." (The PM has parked
  *real* work on a stale-mental-model dup-assertion — asserting "X dups
  Y" from an out-of-date read. Seeing current state stops this.)
- **Age / last-touched / who-touched is invisible** until a re-read.
- **Per-Epic slice-coverage is guesswork** — can't see an Epic's
  slice-set + which are done vs open, so "is this Epic fully captured
  or thin?" is unanswerable at a glance.

**Design implication:** icebox as its own clean list; staleness/age
signals at a glance; overlap/dup hints; per-Epic slice-coverage.

### 3. Order & schedule
**Today:** `move` (icebox→ready, `--position`) to order the ready
queue; split when too big; compose an iteration via `iteration create`
+ `epic`/add-member per story + `designate` + promote.

**Pains**
- Orders the ready-queue **blind** — sets `--position` without seeing
  the ordered list being produced. Owns the order, can't see it.
- Iteration compose is a **multi-verb dance** with no single "here is
  the iteration I'm composing" view — confirmed only by re-reading.
- The **PLAN-gate is all in the head** — stories in one iteration must
  be mutually independent / parallelizable (deps belong in a *prior*
  iteration); nothing shows dependency structure.
- **Splitting without an Epic slice-spectrum view** — can't tell if a
  split set covers thinnest→richest or leaves a gap.

**Design implication:** see the ready-queue *as* an ordered list while
ordering it (direct manipulation); a single iteration-composition view;
surface dependency structure for the PLAN-gate; Epic slice-spectrum for
splitting.

### 4. Plan with the owner
**Today:** Ryan asks "how's the backlog?" → PM status-dump.

**What Ryan needs to SEE** (he sets WHAT/priority; PM owns the HOW of
ordering):
- The ready-queue **in priority order** — what's next, top-down.
- The icebox **grouped/triageable** so he can weigh in on what to
  promote.
- The **3-iteration planned horizon** (N / N+1 / N+2).
- **Per-Epic** outcome + slice-set + progress — the arc, not loose
  stories.

**Settled constraint:** planning happens through the *agent
conversation* (no chat-surface in scope). The **view is the shared
artifact** they point at while they talk — so it must carry enough that
Ryan isn't asking the PM to read it aloud.

**Design implication:** the view must be self-sufficient as "the
answer" to "how's the backlog?"

## Net-new vs. build-on  *(sharpened against the Roadmap renderer)*

The Roadmap renderer was re-grounded firsthand (`Roadmap.tsx` +
`selectRoadmap.ts` + `deriveOutcomeDrillIn.ts`); the complement-boundary
is precise.

**What Roadmap covers (verified):** Now/Next/Later horizon bands;
outcome cards (name · why · readiness · tone); outcome direct-
manipulation (drag-reprioritize within a band + a re-horizon picker);
drill-in outcome → its iterations → per-iteration *readiness*
(ready/next/gap/not-started) + convene-checks + a "next to convene"
banner. **Deepest granularity = iteration readiness — it stops there,
no story records.**

**The seam (clean, no overlap):** Roadmap drills outcome →
iteration-*readiness* (rolled up); the backlog drills iteration → its
*stories* (the detail). They MEET at the iteration but at different
granularity. **The line the backlog must not cross:** never re-draw the
Now/Next/Later outcome cards (Roadmap owns them) — the backlog picks up
*below* the iteration, at the story-ledger.

**Backlog's exclusive lane** (Roadmap doesn't touch): individual story
records / the ledger · the icebox · ready-queue story *order* ·
per-story state (Dev/DeskCheck/Delivered) · age/staleness/dup signals ·
dependency structure · authoring · the Epic slice-set at *story*
granularity.

**Build-on — proven chrome to reuse, not rebuild:**
- **Ordered-ready-queue direct manipulation** — the "see the queue as a
  visible ordered list *while* ordering it" pain already has proven
  chrome: `SortableList` + `DragGrip` + `useDragReorder` + a
  reprioritize action, running today on Roadmap's outcome-reordering.
  The backlog ready-queue ordering is **net-new wiring** (a
  story-reprioritize over that same list) — *not* net-new UI.
- The board panel + `BoardRoadmapSwitcher` chrome; `view_backlog` data.

**Genuinely net-new:** icebox-as-clean-list + triage signals
(age / staleness / dup); the iteration-composition view; the Epic
slice-spectrum view; authoring-with-the-backlog-visible.

## Open for the Ryan + designer synthesis

- Surface relationship / placement (third peer in the
  Board↔Roadmap switcher vs. its own roomier surface) — deferred until
  the jobs are agreed; placement serves the jobs.
- First-slice scope (read-only ledger split first vs. ledger +
  authoring together) — PM files after mockup review.
