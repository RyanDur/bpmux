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
