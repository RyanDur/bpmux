# Story lifecycle as a type (Designing with Types)

Status: **design, converging with the owner** (2026-07-05). Not yet built.
This is the core-model arc that replaces the flat `BoardState` enum + the
`is_closed` flag with a lifecycle type that makes the illegal states
unrepresentable. Read `architecture-rules.md` (two grains, shared-coarse vs
local-fine) first; this is downstream of it.

## Why

`Story` today carries its board position as `BoardState`
(`Icebox | Ready | InProgress | Acceptance | Accepted | Retired`) plus a
separate `is_closed: bool`, plus state-specific facets as *optional* fields.
That shape lets several illegal states be represented, each currently held
together by hand-maintained invariants and runtime discipline:

- `is_closed` must equal `state ∈ {Accepted, Retired}` — maintained by pairing
  every `StoryClosed`/`StoryReopened` with a terminal `StateTransitioned`. A
  bool that *shadows* position.
- `acceptor: Option<Acceptor>` is meaningful only from acceptance onward — but
  the type permits an acceptor on a Ready story.
- the fine middle stage is `placement: Option<StageId>`, meaningful only when
  in-progress — a `Some` that is valid only sometimes.
- the retire reason has no home (it rides `StoryClosed { reason }`, an event,
  not the state).
- "is this story concluded?" is a hand-written predicate over enum variants
  (`is_concluded()` / `is_closed`), not a `match`.

The recurring tell — reaching for a predicate to group variants, and
maintaining an invariant by discipline — is the Designing-with-Types nerve.

## The principle (owner, 2026-07-05)

A distinction earns its own type **not by a count of differing fields** but
when conflating the states either (a) makes an illegal state representable, or
(b) erases a rule the compiler could otherwise enforce. One rule-bearing
difference is enough; zero cosmetic ones never are. By that test the three
board columns qualify — they admit *different operations* (pickup / deliver /
accept-or-reject) — even though most `Story` data is state-independent.

## The type

```rust
enum Lifecycle {
    Backlog,                       // icebox — filed, not yet on the board
    OnBoard(OnBoard),              // the three shared board columns
    Concluded(Conclusion),        // has left the board
}

enum OnBoard {
    Ready,                        // a bare position; order lives on iteration membership
    InProgress(StageId),          // the OPEN column: always at some configurable middle stage
    Acceptance(Acceptor),         // carries its acceptor (team | owner | dev)
}

enum Conclusion {
    Accepted(Acceptor),           // who accepted it — drives the dev-accepted badge
    Retired(RetireReason),        // the retire rationale (String today)
}
```

`Story { id, title, body, comments, order, parent_epic, …, lifecycle: Lifecycle }`
— the ~90% state-independent data stays flat on `Story`; only the
state-specific facets move into their variant.

### The three columns, resolved

- **Ready** — no state-specific data. A bare variant.
- **In-progress** is the *open* column: a story is in-progress *at* a position
  in a middle the instance defines (configurable 1..N). So it carries a
  `StageId` (interned, configurable — not a fixed `Dev | DeskCheck` enum). This
  is the "definable internal state." It kills `placement: Option`.
- **Acceptance** carries its `Acceptor`. Verified consumers: the accept/reject
  gate class (`accept.rs`, `reject.rs`, `pass.rs`), `owner_park` (only for
  `state == Acceptance`), and the wire/badge. It kills `acceptor: Option`.

### Acceptor scope, resolved (substrate-checked)

`Story.acceptor` is read both *during* acceptance (gate class + owner-park) and
*on accepted stories* (`view/board/story.rs:46` emits it on the wire; the
`agent-cli-dev-accepted-badge` e2e shows a dev-accepted badge). So the acceptor
**rides from `Acceptance(Acceptor)` into `Concluded::Accepted(Acceptor)`** — it
is not acceptance-only. `Retired` carries a `RetireReason` instead; there is no
acceptor on a retired story.

## What it kills, structurally

- `is_closed` and the `is_closed ⟺ terminal` invariant — concluded-ness *is* the
  `Concluded(_)` variant.
- `placement: Option<StageId>` — the stage lives inside `InProgress`.
- `acceptor: Option<Acceptor>` — it lives inside `Acceptance` / `Concluded::Accepted`.
- the homeless retire reason — it is `Concluded::Retired(reason)`.
- the `is_concluded()` / terminal predicates — they become a `match`.

## What STAYS (do not confuse with the above)

- **`StoryClosed` / `StoryReopened` events are load-bearing** and stay. They
  push the actual GitHub *issue* open/closed state
  (`integrations/github/mutation.rs:27,30`). They are NOT redundant with the
  lifecycle — they are how the lifecycle's Concluded/Active edge is mirrored to
  GitHub. Deriving `is_closed` from position does not remove them.
- **Transitions stay config-driven** (`workflow.toml` / `WorkflowConfig`) and
  the gate stays lenient (`check_target_state_valid` only rejects `current == to`).
  The types do **not** own the transition graph.

## The seam: types own state, config owns moves

Division that composes cleanly:

- **Types own** the state, its data, and *which phase am I holding*.
- **Config owns** the moves (which transitions exist, their gate class,
  evidence/CI requirements).

At each handler edge, **parse, don't validate**: match the `Lifecycle` once to
extract, e.g., an `Acceptance(acceptor)`, and from there the accept logic is
compile-time-guaranteed to hold a story that is actually in acceptance; the
config still drives the gate specifics. The lenient `move_story` stays a generic
operation over `Lifecycle`.

## The configurable middle becomes drivable (a feature this unblocks)

Audit finding (2026-07-05): a middle with N>2 stages is only *rendered*, never
*driven*. `deliver` calls `advance(Succeed{ step: transition.from })` with
`transition.from == "in-progress"`, and `pass`/`fail` are pinned to
`"desk-check"` — the verbs advance from the config's *first* and *last* middle
stages, never a story's *actual* intermediate stage. So a story can reach
`code-review` but has no verb to leave it.

With `InProgress(StageId)`, the natural fix falls out: the deliver-class verb
advances from the story's **held** stage (`InProgress(current)`), not a fixed
config source — so an N-stage middle is drivable end-to-end. This is the point
where the configurable middle stops being aspirational. (Persistence for the
fine stage already exists: the de-leak landed `placements.json` + local hydrate,
so the stage survives cold start without a GitHub label.)

## Slice decomposition (expand/contract; renderer LAST)

This arc **subsumes** BD1b's board slice (the fine stage folds into
`InProgress(StageId)`, the placement slice becomes a persistence detail of one
honest lifecycle), the `is_closed` shed, and the wire/decode/renderer. Sketch —
each slice gated + CI-green, behavior-preserving until the deliberate wire
reshape:

1. **Introduce `Lifecycle` + total projections** to/from `(BoardState,
   is_closed, placement, acceptor)` — additive, nothing removed. Prove the
   projection round-trips.
2. **Migrate backend readers** to `match lifecycle` behind the projection
   (handlers, selectors, gate reads), still writing the old fields.
3. **Move the transition verbs** onto the lifecycle (deliver advances from the
   held stage — unblocks N>2 middle); events carry lifecycle deltas.
4. **Contract**: `Story.lifecycle` becomes the source of truth; `BoardState` +
   `is_closed` + `placement: Option` + `acceptor: Option` removed; persistence
   splits the grains at the edge (coarse → GitHub, fine stage → placements.json).
5. **Wire + decode + renderer** — a deliberate reshape. **Owner + designer
   review required** (role boundary: designer owns UX). The wire currently
   composes coarse+fine into a `state` string and emits `gh_state`/`acceptor`;
   the renderer mirrors `columnFor`. This is the only slice that changes what
   the renderer consumes, and it is explicitly held for review.

Guardrail during autonomous work: steps 1–3 (additive/backend, behavior-
preserving) are safe to build; steps 4–5 (removal + wire/renderer) are held for
owner/designer review — they are irreversible and cross the UX boundary.

## Open items for the owner

- Confirm the type shape above (esp. `Concluded::Accepted(Acceptor)` vs an
  acceptance-only acceptor — resolved here as riding into Accepted, per the
  badge; flag if you disagree).
- `RetireReason` — keep as `String`, or model it (e.g. a retire-class enum)?
- Whether to fold the N>2-middle verb fix into this arc (recommended — the
  `InProgress(StageId)` shape is what makes it clean) or ship it separately.
