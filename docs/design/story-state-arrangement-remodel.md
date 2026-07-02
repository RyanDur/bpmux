# Story state × board arrangement — remodel design (#69)

Status: audit complete, decision pending. Dev recommendation at the bottom.

## Thesis under test

"Story sheds `state`; board owns arrangement" — a story's column is the board's
concern, not story identity. The candidate move: `Story.state` leaves the aggregate
for a board-owned `HashMap<StoryId, BoardState>` arrangement slice, potentially
followed by `is_closed`, `ready_order`, and `iteration_slot`.

## What the audit found (2026-07-02, post-raw_labels-shed)

~40 production read/write sites across ~28 files. The split is clean:

- **Arrangement reads** (move-eligible): `board/column.rs` `column_for` — the single
  canonical state→column fold; the `StoryView.state` wire projection; the column
  filters (`list_ready`, `dev_in_flight`, iteration-close blockers); display sites.
- **Workflow reads** (stay near Story): the gate validators already take state **as a
  string** (`workflow/gate/transition.rs`), fed by `.as_str()` at the transition/
  accept/reject/desk-check call sites — the workflow engine never needed to own the
  type. Idempotency guards in move_story.
- **Rollups** (follow the authoritative map): `derive_epic_state`, owner-park — they
  read children's states from the store.

`is_closed` is deliberately orthogonal to state (#505 boundary: close preserves
state; GH close→Accepted happens at ingest). The two fuse only at column/wire time,
so if state moves, `is_closed` must move with it or the fused decision splits across
aggregates.

`ready_order` + `iteration_slot` are textbook arrangement (zero workflow readers)
but persist through a different mechanism (labels, not StateTransitioned) — if they
move, it's a separate later slice.

## The case FOR the shed

State is read as arrangement far more than as identity; the workflow engine proves
ownership is unnecessary; the slice pattern fits (`Arrangement` beside `Stories`);
epic/owner-park rollups would read one authoritative map; raw_labels set the
precedent. A gated path exists (S1 dual-write → S2 arrangement readers → S3 workflow
readers → S4 drop field), and the in-memory label mirror is the proven seam shape.

## The case AGAINST

- State is the subject of a first-class domain event and of #505 close semantics —
  it is genuine aggregate content, not decoration.
- `Stories` keys on `StoryKey` (Saved|Unsaved); an arrangement map can only key on
  `StoryId`. The split introduces a key-space seam and a "story exists but has no
  arrangement entry" bug class that the single struct makes unrepresentable today.
- Every projection gains a join that the single map gives for free.
- At the sync boundary, GitHub persists state as a label ON the issue — the external
  system couples state to story identity; "board owns arrangement" is an internal
  ontology the edge doesn't share.
- The concrete win (the column fold living with the board) is a one-file move that
  does not require shedding the field.

## Riskiest consumer either way

`in_memory/typed_label_mirror.rs` `state_transitioned` writes state in two
representations (GH label + typed) and feeds `stale_state_labels_for` outbox
reconciliation. If an arrangement map and the label mirror drift, the outbox fights
GitHub — silent eventual corruption invisible to single-dispatch tests. Any move
must derive both representations from one source.

## Dev recommendation

Take the 80% now, defer the shed pending evidence:

1. **Relocate the arrangement fold** — `column_for` (+ its bucket vocabulary) is the
   board's; move it under board arrangement and route the scattered direct
   state-comparisons (`list_ready`, `dev_in_flight`, blockers) through it. ~2 files
   of churn, no field moves, captures the ontological point.
2. **Hold S1–S4 (the field-shed)** until the StoryId↔StoryKey join is proven clean in
   practice — e.g. after Story P4 (the Unsaved/Saved discriminated union) settles the
   key-space, which is on the roadmap anyway and would dissolve the strongest
   anti-argument.

The shed is defensible, not urgent; the raw_labels retirement removed genuinely
untyped rot, whereas this trades an invariant for a join.
