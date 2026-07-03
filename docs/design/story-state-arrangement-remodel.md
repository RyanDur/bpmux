# The board remodel (#69) — Ryan's model, written down

Status: model confirmed with Ryan 2026-07-02. This replaces an earlier draft that
audited a shape Ryan never asked for (a story→state map); the mechanical findings
from that audit that still hold are kept at the bottom.

## The ontology

Three concepts, currently conflated into one enum:

1. **Story lifecycle** — what a story IS: open, accepted, closed. `is_closed` and
   acceptance are facts about the story. Closed and accepted stories are not
   representable on the board.
2. **The board** — the representation of CURRENT WORK, and nothing else. The board
   is a projection of the current iteration: adding a story to the board IS adding
   it to the iteration; a story added to the current iteration generally starts in
   the ready queue. There is no board membership apart from iteration membership.
3. **The backlog** — availability: the stories that could be worked. The icebox is
   the pool of possibles inside it. The icebox is not sorted — the PM can organize
   it however they want; arrangement there is a PM affordance, not domain priority.

## The board's shape

- **Two stable states, structural to what a board is: Ready and Acceptance.**
  Beginning and end. Every board has them.
- **One configurable section between them — the "in progress" section — shaped by
  the working human's style.** Ryan's configuration: `in progress`, `desk check`.
  The middle is DATA, defined in a TOML file (board configuration, versioned in the
  repo since it is shared working agreement), not enum variants.
- **The ready queue inherits the iteration's order.** Priority is defined once, at
  iteration composition, by the PM. Ready has no independent ordering of its own.
- Columns are ordered lists of story ids; position is index. Fractional-position
  label math is an EXPORT encoding at the GitHub edge, never domain.

## What the current code gets wrong about this

- `BoardState` inverts both halves of the model: the stable end-state carries the
  retired name (`Delivered` — renamed to Acceptance long ago, but only as a renderer
  display label at `board/types.ts:104`; domain, wire, bucket headers, and MCP
  output still say delivered), while Ryan's configurable working-style stages are
  frozen into the type system as `InProgress(Dev | DeskCheck)` and matched on by
  every layer.
- Loose-ready (stories on the board without iteration membership) exists in
  `list_ready` today; under the model it is a smell — board membership IS current-
  iteration membership.
- `ready_order` fractional labels are a parallel ordering mechanism that contradicts
  iteration-inherited order. (`list_ready` already half-implements the model: it
  orders by the current iteration's `stories` sequence first.)
- Icebox arrangement (`IterationSlot`, typed 2026-07-02) carries no domain meaning —
  it is the PM's free organization of the possibles. It stays as an affordance;
  nothing may treat it as priority.

## Process-owned points (deliberately NOT hardened into the domain)

- **Rejection routing** is a process question and has already changed once: it began
  as move-to-top-of-ready and evolved (with iteration-as-collection) to move-to-next-
  iteration. The code should route rejection through one policy point that process
  can change, not bake either answer into the state machine.

## Slice sequence (each gated, in canon order)

1. **Naming-first: `Delivered` → `Acceptance`** through domain and wire. The GitHub
   label string `"delivered"` stays at the edge as codec vocabulary — the
   anti-corruption layer exists so the domain speaks the product's language while
   GitHub keeps historical strings. Transitional note: the enum briefly holds both
   `Acceptance` (the column) and `Accepted` (the lifecycle fact) until slice 4.
2. **Board configuration TOML** — reuse `.bpmux/workflow.toml` (Ryan, 2026-07-02):
   ONE config defines both the workflow state-machine and the board's columns; no
   parallel board.toml. The loader (`workflow/loader.rs`) already reads it and falls
   back to `bundled_default`; its `in-work` section already holds the configurable
   middle (`in-progress`, `desk-check`). The board just doesn't read it for columns
   yet — today columns come from the hardcoded `BoardState::InProgress(Stage::Dev |
   DeskCheck)` enum, a parallel representation. Slice 2 connects them. Gated
   sub-sequence (dev's HOW):
   - **2a** — reconcile the section skeleton: `acceptance` moves `in-work` →
     `post-work`, so `in-work` is EXACTLY the configurable middle (Ryan's model:
     Ready and Acceptance are stable bookends; the configurable part is strictly
     between them). Skeleton-safe (in-work keeps in-progress+desk-check).
   - **2b** — backend derives the board's middle-stage vocabulary (ordered ids +
     labels) from the config `in-work` section; the CLI board headers and column
     bucket vocabulary read it instead of hardcoding. Behavior-preserving (default
     config == current).
   - **2c** — the wire serves the board stage config; the renderer's GROUP_ORDER
     middle + columnFor read it. Behavior-preserving.
   - **2d** — falsifier: reconfigure the `in-work` labels/order in a test; the board
     columns follow (proves it's config-driven, not still hardcoded).
   Generic-engine-in-app canon: the board engine is baked in, the working-style
   config lives in the light layer (the TOML). Ryan (2026-07-02) chose to FOLD this
   into the genuinely-open middle — slices 2 and 3 are one arc.

   ### Slice 2 arc — the open middle (audit-resolved plan)
   Type: `BoardState::InProgress(StageId)` where `StageId(Arc<str>)`. The rails
   (`Icebox|Ready|Acceptance|Accepted|Retired`) stay typed variants; only the middle
   opens. Rejected `StageIndex(u16)` — persistence (JSON + SQLite outbox) and the
   wire `StoryView.state` are ALREADY id-strings, so `StageId` is isomorphic to the
   on-disk format → migration-free; an index breaks on config reorder. `Arc<str>`
   keeps `.clone()` a pointer-bump; dropping `Copy` on `BoardState` is a small
   ripple (~5 production sites: `TargetState` [drop Copy, `&self` methods],
   `Story::state()`, a few double-use sites — all durable containers are already
   `Clone`). Workflow gate needs ZERO change (already string-keyed vs `.as_str()`).
   The 8 exhaustive `Stage::Dev|DeskCheck` destructures (`as_str`, `github_label`,
   `parse`, `replay_transitions`, `state_ordinal`, `column_for`,
   `current_state_display`, get_story state-label) centralize onto config lookups.
   RISKIEST consumer: `state_from_labels` (github/story_decode.rs) — priority-ordered
   decode with hardcoded alias labels ("needs fixes"→first-in-work, "rejected"→ready,
   "ready for accept"→acceptance) that have no config home yet; needs a round-trip
   property test over the config middle table.

   Gated sub-slices:
   - **S0** — seam: add `StageId(Arc<str>)` + a `Stage → StageId` bridge; a config
     lookup returning today's two bundled middle stages. No public type change,
     existing tests untouched.
   - **S1** — flip `InProgress(Stage)` → `InProgress(StageId)`, drop `Copy`;
     centralize the 8 destructures onto config lookups; fix `TargetState`,
     `Story::state()`. Middle still populated by the two bundled stages →
     behavior-identical; fixtures rewrite `Stage::Dev`→`stage_id("dev")`. Big diff,
     no behavior change.
   - **S2** — middle becomes config-defined (add/remove/rename actually work):
     `workflow.toml [in-work]` ordered list feeds the middle table; `ColumnBucket`
     middle → `Middle(StageId)`, de-exhaust the two CLI matches; retire the `Gate`
     closed-enum coupling in `replay_transitions` toward gate-id strings. Falsifier:
     a test config with a THIRD middle stage; stories can hold it.
   - **S3 / S4** (wire column-structure message + renderer consumes N columns) — the
     HINGE: fold these into the board-as-projection arc rather than a separate
     follow-up, because they're where "Story sheds state" would otherwise force a
     rewrite of the just-added `StageId` field. Design `StageId` AS the projection's
     stage identity to avoid double-touching `Story.state`.
3. **Board = ordered projection of the current iteration**: columns as ordered
   lists; ready inherits iteration order; adding to the board = adding to the
   iteration; loose-ready dissolves. `ready_order` fractional machinery reduces to
   GitHub-edge export encoding, then dies when nothing at the edge needs it.
4. **Lifecycle split**: accepted/closed leave the board vocabulary entirely; the
   board never holds them, views derive "done" from story lifecycle.
5. **Rejection policy point**: current behavior (move to next iteration) expressed
   as the single replaceable policy.

TOML placement: RESOLVED — `.bpmux/workflow.toml` (the existing loader path),
reused as the single board+workflow config. Versions with the shared tree.

## Retained mechanical findings from the earlier audit

- ~40 production `story.state` read/write sites across ~28 files; the workflow gate
  validators already take state as a string parameter, so Story never needed to own
  the type for validation.
- Riskiest consumer for any state move: the in-memory adapter's label mirror
  (`typed_label_mirror::state_transitioned`) feeding `stale_state_labels_for` outbox
  reconciliation — the arrangement representation and the GH-label mirror must
  derive from one source or the outbox fights GitHub silently.
- The `StoryKey`/`StoryId` seam: board lists reference `StoryId`; the stories map
  keys on `StoryKey` (Saved|Unsaved). Under the board-as-iteration-projection model
  absence from the board is MEANING (backlog or done), not corruption — the
  remaining referential edge (list members exist in the stories map) is small and
  shrinks further after Story P4 settles the key-space.
