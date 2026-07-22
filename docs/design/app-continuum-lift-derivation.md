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

## Station log catch-up (2026-07-21) — through mcp_tools

The doc fell behind the lift; commit messages carried the narrative.
Recorded here so the doc stays the map. Landed CI-green since the
storage re-derivation, in order: comprehension `8755b34f` → memory
`0aea0c39` → product_management `cafbc2c1` → board `add6a82c` + view
projections `2c8033b9` → reactions `57f2cfaa` → awareness `f73e778b`
(the hub gate cleared) → roster `bc827eb1` → owner_session `499df259`
→ source_sync `22d8c66a` → storage `255b758d` (the re-derived order
proved out: domains first, edge last) → source_codec `25a7e639` →
rate_limit `427cb73a` → ci `13cb3b8b` → integrations `c551b3e4` →
local_url `890655d0` → protocol `af2d7493` → self_update `ab677dd8` →
electron_capture `689bf8dc` → unfurl `88c295c8` → hub `dbcc5ab5` →
store `f4cc0a41` → mcp_tools `75e72b31`. Thirty component crates.

mcp_tools (2026-07-21): the app's mcp module surveyed as the strongest
remaining leaf — 1,648 LOC, zero crate:: references (full-text grep,
per the hub census lesson), one consumer (bridge tool_registry), one
identity: the MCP tool-schema catalog (every file an rmcp Tool
builder). Named by mechanism because `mcp` is reserved for the future
application shell. Shimmed as `pub use ::mcp_tools as mcp;` — no
consumer touched. rmcp promoted to workspace.dependencies on its
second consumer.

What remains in applications/bpmux: bridge (~27k), transport (~13k),
sync, room, store composition residue, server + cli shells, and small
leaves — telemetry (630 LOC, blocked only on the open
diag-consolidation question), location, the ci store-slice remnant,
auth/bookmark/briefing. The write-path tail (bridge, transport, sync)
still waits on the store↔sync and store↔transport cycle cuts; the
sync→server composition reach cuts with them. Then the fat app splits
into the four thin shells and the repo reclaims the bpmux name.

## The write-path tail re-derivation (2026-07-21)

Stations landed since the catch-up: telemetry `efb58970` (the diag
consolidation question closed NO — different identities) → location
`630026a0` → auth+bookmark+briefing `49b528f4` → room `49146197` →
the CI watch binding re-homed to store::ci_watch `b7d31bce` → sync +
store_test_support `a3efa5a2`. Thirty-eight component crates.

Two firsthand surveys of the tail (bridge; transport/sync/store/room)
revised the doc's blocker model:

- **The store<->sync and store<->transport cycles are dead.** The
  store station left `pub use ::store::*` in the app, so every
  production "cycle edge" already resolves to the extracted crate.
  What remained were test-seam reaches into server::store_wiring —
  cut by moving the drain listener test beside the wiring and
  graduating SeededStore into components/store_test_support.
- **The unnamed real blocker is transport->bridge** (tool_dispatch,
  board_acceptance, roadmap_write_handlers name bridge handlers), so
  bridge extracts before transport, not after.
- **bridge's own blocker was crate::sync::SyncOrchestrator** — gone
  now that sync is a crate. bridge decomposes as: pure domain gates
  (atp_substantive_gate, deliver_story_body_diff_gate,
  chat_relevance) -> product_management; a clean MCP-edge cluster
  (tool_registry with DISPATCHED_TOOL_NAMES, rpc_client,
  event_subscription, connection, view_model, mcp_handler); then
  tools_stories/move_story/dispatch/tools/polling. One edge->edge
  inversion to cut on the way: bridge/dispatch naming
  transport::rest::iteration_close.

Remaining order: bridge (gates -> edge cluster -> handler tail) →
transport → the four thin shells. Owner rulings recorded 2026-07-21:
mcp and cli affordances stay 1:1 (the parity audit in cli is the
lock; it re-homes where both catalogs are visible at the shell
split, and the verb inventory becomes a real cross-door contract
component when both doors derive from it); databases/ is deferred —
Ryan pairs on it after the lift.
