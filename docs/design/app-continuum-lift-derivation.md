# App Continuum Phase-2 lift — derivation and verdict

Derived 2026-07-08 (dev), after SC completed — the boundary locks that
draw the seams landed at `a4edc54e`, which is what this derivation was
deliberately sequenced behind. Reference: dam5s kotlin-monorepo-example
(applications / components / databases / protocols), corrected by the
primary source's anti-support stance (support is the narrow exception,
never a bucket; mechanism names only).

## Verdict: derivation now, execution HOLDS

The lift's seam map is derivable today and is recorded below. Executing
the crate split now would be premature promotion, for three
evidence-backed reasons:

1. **Stage and friction.** bpmux is v3 (feature-grouped single-crate);
   architecture.md's own progression rule is "progress when friction is
   urgent, not speculatively," and no lift-justifying friction is
   currently live: no compile-time pain is named anywhere, ownership
   seams are held by the ten `*/boundary_tests.rs` fitness locks, and
   the renderer type-duplication friction's named fix is codegen, not a
   crate. The locks are the classic pre-v4 state working as designed.
2. **In-flight seams.** The gated write-path slices — conflict
   machinery retire-or-finish, the evidence re-home (the gate-time
   file-read at bridge/tools_stories/acceptance/helpers.rs is a live
   store-bypass), and the drain trigger action-to-state re-home — cross
   exactly the pm/sync/integrations/bridge boundaries the lift would
   freeze into crate APIs. Lifting first means re-cutting crate
   boundaries after each slice, and would ossify the evidence anomaly
   into a public seam.
3. **Menu-derived groups fail the derive test.** A `databases/`
   top-level tree for one database with one migration file, and a
   `protocols/` tree for one wire consumer, are the reference example's
   shape, not this codebase's need. They earn existence at >1 database
   or a second wire consumer.

## The derived seam map (locks are the evidence)

Applications (settled): server / electron / mcp / cli — thin
deployables.

Component extraction order, when friction arrives, derived from which
modules already have locked boundaries, zero back-references, and their
own domain:

1. `message` — support leaf, imports nothing feature-owned; the most
   protocols-shaped module in the tree.
2. `workflow` — guard-rails leaf; the workflow/pm cycle is closed and
   locked.
3. `kernel` — the redux engine mechanism; a genuinely-shared named
   mechanism, the anti-support rule's legitimate exception.
4. `storage` — edge; rusqlite/refinery confined by its lock; carries
   the database-mechanism seam (migrations stay with it until a second
   database exists).
5. `view` — projection over domain data only (its remaining
   derivations continue migrating into domain selectors; it shrinks
   before it lifts).
6. `integrations` — store-free port adapters (locked as of SC-4).

Last, and only after the write-path slices land: product_management,
sync, store, bridge, transport — the WR churn lives there.

## Preconditions for execution (re-derive nothing; just check these)

- Conflict machinery decision executed (retire or finish).
- Evidence re-homed off the gate-time file-read.
- Drain trigger re-shaped (pending queue to store slice).
- A named, concrete friction the locks cannot hold (compile-time,
  test-isolation cost, deploy independence) — the trigger the
  Continuum's own rule requires.

First execution step when triggered: extract `message` (or `workflow`)
alone as a v4 station — one app over first components — per the primary
source's warning that the v4-to-v5 transition shouldn't be taken
lightly. The four-application v5 shape follows only on multi-deployable
friction.

## Debt noted at sight

architecture.md's stage section (verified 2026-04-18) predates the
restructure: it lists seven top-level modules where src/ now holds ~30
feature modules, and its friction list predates the fitness locks. It
needs regrounding against the post-restructure tree when next touched.
(Paid 2026-07-09: architecture.md reground landed `abb3f337`.)

## Execution (2026-07-10) — the lift runs as backend work

Ryan, 2026-07-09, direct: "the app continuum restructure is also a part
of the backend." The HOLD above was a readiness check, not an exit — the
lift executes station-by-station as backend work, each station CI-green
before the next. The three write-path preconditions all landed (conflict
machinery retired `343e714f`, evidence re-homed off the gate-time
file-read, drain trigger state-shaped `94c3928b`); the friction trigger
is superseded by Ryan's call. The github twin pre-lift blocker cleared
`f6e623e4`.

**Station 1 = `kernel`**, chosen over message on survey evidence
(2026-07-10): kernel has zero parent-crate imports (message reaches
`crate::ci_link`, making it a two-module extraction), one external dep
(tokio) versus four, ~25 consumer files (20 in store/) versus ~108
across 17 modules, zero integration-test coupling, no orphan-rule
surface (generic over S/A, names no domain types), and pm's boundary
lock already declares it the sanctioned leaf. Post-K-1/K-2 the kernel is
a finished, pure mechanism — the strongest first station.

Station 1 also establishes the workspace pattern every later station
reuses: `[workspace.package]` (version/edition/license),
`[workspace.dependencies]` (promote a dep when a second consumer
appears — derive, don't menu), and `[workspace.lints]` (the per-package
deny-lints would otherwise silently not bind new crates), plus the
pre-push size gate's file roots broadened to `components/*/src`.

Remaining seam order after kernel: message (+ci_link) → workflow →
storage → view → integrations; write-path modules (product_management,
sync, store, bridge, transport) last. Re-derive nothing per station —
survey the seam firsthand, spec, land CI-green.

## Stations landed + the storage re-derivation (2026-07-10)

Landed CI-green: kernel `1b0d4bdc` → message (+ci_link inversion)
`d9ac4074`+`91fd0585` → diag+workflow `4c077138`+`a61382ff`. Five
component crates; the workspace pattern (workspace.package /
dependencies / lints, size-gate roots) holds.

**The storage station as ordered above was wrong, and the survey that
grounded it says why:** since the five-ports inversion, storage is an
EDGE that implements five domain-owned port traits
(awareness::SurfacedTopicsRepo, log::MessageLog, memory::LessonRepo,
sync::OutboxRepo, routing::UserRepo) and consumes their DTOs. A
storage crate would need those domains as dependencies — today that is
a path-dependency back into bpmux, a cycle. A "ports crate" would be a
menu-derived bucket that dismembers the domains. The honest order
extracts the port-owning feature domains first, each a real
feature-first component:

1. `log` — zero unextracted imports (message only). Package name must
   NOT be `log` (crates.io logging facade is in the lock, ~25 edges) —
   ship as `message_log`.
2. `persist` (28-LOC PersistError leaf, zero crate:: refs) then
   `routing` — routing's only unextracted import is PersistError.
3. `comprehension` (message-only leaf) then `memory` — memory's only
   unextracted import is comprehension::classifier.
4. `awareness` — WAITS: it threads crate::hub::Hub through all five
   detectors; gated on the hub story, lands after memory.
5. `storage` — after sync's port (OutboxRepo/QueuedEvent) is
   extractable, i.e. rides the write-path phase. Its migrations/
   refinery embedding travel cleanly with it (verified: no path arg,
   CARGO_MANIFEST_DIR-relative). view and integrations re-derive the
   same way when reached — both are projections/adapters OVER domains
   and follow their dependencies, not the original menu order.

The two storage boundary locks (super-trait-only composition;
rusqlite/refinery confined to the edge) stay LIVE in bpmux until
storage extracts — they read feature dirs by path and re-home with the
moves that affect them.
