# The message split — chat protocol vs state-broadcast protocol

Status: designed 2026-07-03. Executes the "`message` split (domain type vs wire
DTOs)" item of `app-continuum-restructure.md` Phase 0. Grounded in a firsthand
lifecycle survey of every `Message` producer, consumer, persistence path, and
the renderer's decoders.

## The finding: `Message` is two protocols in one struct

**Protocol A — the persistent chat log.** `id / from / sender_id / text / ts /
message_type∈{Chat,Activity,System,Insight,Nudge} / to / addressee /
explicit_privacy` (+ `multi_depth` as chat-line elaboration on CI activity
broadcasts). Durable in the `messages` table, recipient-filtered, history-
hydrated, searched, classified, recalled. ~85% of the ~38 non-test consumers
touch only these fields. This is the product's unit of team communication.

**Protocol B — the ephemeral state broadcast.** `message_type∈{StateDelta,
StateSnapshot}` + `state_delta` / `state_snapshot` payloads, empty `text`.
Semantically a redux-store projection frame: snapshot on connect
(`store/snapshot.rs`, never persisted), deltas as change events. It rides the
chat struct because the chat frame was the available truck — and that
mechanical abuse is the direct cause of three warts:

1. **Accidental durability.** Whether a delta persists is decided by which
   method the caller happened to pick (`session.broadcast` persists;
   `broadcast_ephemeral` doesn't) — durability is a call-site accident, not a
   type property. CI/roster/reactions/BoardInvalidated deltas persist into the
   chat table purely as a reconnect-replay cache: derived store state stored
   durably beside chat — a rogue stored-shadow of the one store.
2. **The migration purge.** `Stories` deltas were once persisted, later
   reclassified ephemeral; `storage/session/migrate.rs` carries a permanent
   special case deleting the stale rows.
3. **The tolerant deserializers.** `state_delta`/`state_snapshot` parse via
   `from_value(..).ok()` only because evolving wire DTOs land in the durable
   chat blob — one renamed variant would otherwise poison every history read.

The wire framing compounds it: `transport/protocol.rs::ServerMessage` is the
real envelope (serde `tag = "type"`), but chat and state both ride the ONE
`ServerMessage::Message` variant, discriminated by an inner `message_type`
string. A Chat message that carries a `state_delta` is representable today —
an illegal state the types permit.

## The boundary violation (why this is a Phase-0 item)

`message` is a SUPPORT component, yet `state_payloads.rs` imports features:
`CiSnapshotWire.state: crate::ci::CiState` (:149), `RosterMemberWire.role/
presence: crate::store::{RosterRole,Presence}` (:164-165 — the types actually
live in `roster/`, re-exported via `store`), and `to_subscription_event()`
calls `crate::ci::github_actions_url()` (:61). Support→feature, backwards.

**Why the obvious splits are impossible in Phase 0.** `Message` embeds the
`StateDeltaPayload` union (a serde `tag="domain"` union over every feature's
delta). Feature-owned wire types ⇒ the union imports all features: in
`message` that is support→feature ×5 (worse); in `transport` that makes
`Message`→`transport` (support→edge). The full frame split (below) changes
the wire and persistence semantics — a protocol redesign, not a behavior-
preserving relocation. **The only direction-clean, wire-identical Phase-0
shape: make `message` a self-contained leaf** — the shared wire vocabulary
that features convert INTO via `From` impls they own (feature→support, the
codebase's existing idiom in `ci/slice/listeners`, `roster/listeners`,
`reactions/slice/listeners`).

## Phase 0 — leaf-ification (now; every slice CI-green, wire JSON byte-identical)

**S1 — pin the wire first (tests only).** Exact-value golden tests in
`message_tests.rs` for `StateDeltaPayload::Ci` (each `CiState` string),
`Roster` (each role/presence string), and a snapshot; historical-fixture
re-reads (persisted-shape JSON missing `url` etc. must still parse — the
tolerant deserializer's silent-swallow is the stealth risk of the whole arc);
a `to_subscription_event` golden pinning `ci_state_changed` exactly.

**S2 — `ci_link` support leaf.** `github_actions_url()` (+`GITHUB_BASE_URL`,
`resolve_ci_actions_url`) is pure env/config composition with zero ci-domain
types (`ci/broadcast.rs:59-84`). Move verbatim to `src/ci_link.rs`; `ci`
re-exports flip so zero callers change; `state_payloads.rs:61` calls
`crate::ci_link::`. Identical function, identical call moment, for both live
and replayed deltas — byte-identical by construction.
  *Rejected cure:* reading the already-populated `snapshot.url` field — its
  `#[serde(default)]` means pre-field persisted deltas replay with
  `url: None`, changing the subscription-event JSON on the replay path. That
  shape is correct only in Phase 1+, when replay derives from the store.

**S3 — `CiStateWire` mirror.** In `message`: `enum CiStateWire` with
`rename_all = "snake_case"`, `Default = Unknown` — byte-identical to
`ci::CiState` (renderer pins the strings in `stateSyncDecoders.ts:15`).
`impl From<CiState> for CiStateWire` lives in `ci/mod.rs`, exhaustive match,
no wildcard — a new domain variant fails compile at the conversion. Serde-
equality drift test per variant. Four construction sites flip to `.into()`.

**S4 — `RosterRoleWire` / `PresenceWire` mirrors.** Same treatment; mirrors
deliberately have NO `rename_all` (PascalCase on the wire — `"Blocked"`,
pinned in `stateSyncDecoders.ts:50-51` and `store/stores_tests.rs:514`).
`From` impls + drift tests in `roster/`.

**S5 — file split + fitness.** Pure structural decompose of `message/`:
`chat.rs` (Message/MessageType/tolerant field deserializers), `wire.rs` (the
renderer-facing DTO vocabulary incl. mirrors), `state_delta.rs` (the union),
`state_snapshot.rs`, `multi_depth.rs`, `subscription.rs`
(`to_subscription_event` + `payload_matches_filter` — the file Phase 1
deletes wholesale). Public `crate::message::*` surface unchanged. Plus
`message/boundary_tests.rs`, same fs-walk mechanism as
`product_management/boundary_tests.rs` but **default-deny**: strip the
allowlist (`crate::message`, `crate::ci_link`) then assert no `crate::`
remains — an allowlist for a leaf never drifts, and strip-first sidesteps the
`crate::ci`⊂`crate::ci_link` substring hazard. Ships with the
`docs/architecture.md` module-map touch-up (per the restructure doc's rule).

S1 first; S2/S3/S4 independent; S5 last.

## Phase 1+ — the frame split (deferred; requires renderer coordination)

The end-state, recorded so it is not re-derived: state broadcasts become their
own `ServerMessage` variants (the envelope already exists — stop overloading
`ServerMessage::Message`); `Message` drops the payload fields and is chat
alone; reconnect replay derives from the redux store (snapshot + deltas-since)
instead of a persisted copy in the chat table; persistence becomes a type
property, not a caller choice. That deletes: the tolerant deserializers, the
`is_ephemeral_stories` migration purge, the `state_delta.is_some()` chat-table
scans, and `subscription.rs`'s emit-time mapping (cure (a) becomes correct).
Sequencing: alongside the verdict collapse / WS-channels work, when the frame
semantics change anyway. Illegal states (chat carrying state payloads) become
unrepresentable then — Phase 0 deliberately does not attempt it.

## Explicitly out of scope

- The five `serde_json::Value` union variants (stories/epics/outcomes/
  iterations/roadmap) — pre-existing typed-wire debt; not expanded, not fixed
  here (they type up with the read-model work).
- `store/snapshot.rs`'s composition of feature types — that is the
  store↔domain seam (restructure violation #5), a separate Phase-0 item.
- Whether `ci_link` folds into a broader config/github component at the
  Phase-2 lift — decide at the lift.
