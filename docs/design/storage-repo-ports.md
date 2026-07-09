# Storage inversion — per-feature repo ports (restructure violation #3)

Status: designed 2026-07-03 (two firsthand passes: god-trait surface map + port
decomposition). Executes the "storage per-feature repo ports" item of
`app-continuum-restructure.md` Phase 0. Sister to `message-protocol-split.md`.

## The violation

`storage/mod.rs` defines two god traits — `Storage` (session-scoped, 26 methods)
and `GlobalStore` (global, 11 methods) — and imports six+ feature modules purely
to name their types in method signatures. The persistence *contract* is dictated
by the storage EDGE; feature-first wants each feature to declare the repo PORT it
needs and storage to IMPLEMENT it. The smell is not the direction (storage→feature
is the legal "edge serves feature" arrow) — it is the single trait that *defines
every domain's persistence methods* in one edge module.

## What the ground truth changed

**~40% of `Storage` is dead or test-only trait surface** — production reaches those
concepts through free functions, never the trait:

| Method(s) | Real production home | Trait-consumer |
|---|---|---|
| `search_messages` | `read_messages()` + `comprehension::search` (`rest/search.rs`) | test only |
| `load_state`/`save_state` | **LIVE — `storage/data/migrate.rs` migration flags** (not dead; the survey missed it) → StateKv stays storage-owned | migrate.rs (prod) |
| `dir` | `FileStorage`/`SqliteStorage` `dir` field | test only |
| `load_bookmark`/`save_bookmark` | free fns `bookmark::{load,save}_bookmark` (file-path, backend-bypassing) | test only |
| `load_reactions`/`save_reactions` | free fns / reactions store; `save_reactions` only at `transport/test_seed.rs` | test-seed + tests |
| `save_lessons` | — | no non-test caller |

**`GlobalStore` has ZERO production wiring** — every caller (identity, briefing,
telemetry, auth) uses the underlying free functions directly; both impls are thin
pass-throughs. It is a test-only abstraction.

**`PersistError`/`QueuedEvent` already live in `crate::sync`.** Storage reaches them
only through the fossil re-export `product_management::board::orchestrator::*`
(`board/mod.rs` `pub use crate::sync as orchestrator`) — a path each moved port
rewires to real `crate::sync`.

**Three live impls**: `SqliteStorage` (delegating to `sqlite_*` submodules),
`FileStorage` (the *default* backend — `backend.rs` maps anything but `"sqlite"`
to File, and sqlite falls back to File on open error), and the
`InstrumentedStorage<S>` timing decorator. Every port must be implemented in all
three. The dual-backend `contract!`/`global_contract!` suites
(`storage_tests/*`) pin behavior across both backends — the safety net for every
slice.

## The decision: prune the dead, port the five real

**Prune** (pure deletion + repoint the ~1 test-infra caller, trim the contract
test): drop `search_messages`, `dir`, `load/save_bookmark`, `save_lessons`, and
(pending the trace below) `load/save_reactions` from the `Storage` trait. This
deletes storage's coupling to `comprehension`, `bookmark`, and `reactions` — no
port manufactured for code nothing calls. `load/save_state` STAYS (migrate.rs).

**Reactions caveat (trace before pruning):** `save_reactions` is called by
`transport/test_seed.rs` (the e2e seed endpoint) but `load_reactions` is
test-only — so it is unresolved whether reactions are backend-persisted-and-read
anywhere in prod, or the whole trait path is vestigial. Trace the reactions
persistence lifecycle firsthand (does any prod startup/listener read
`load_reactions`? how does the seeded state reach the store?) BEFORE deleting;
if the seed path depends on the round-trip, repoint `test_seed` to whatever the
real reactions store is, or keep reactions as the one exception. Do not delete on
the survey's assumption.

**Port** the five load-bearing ports, each defined IN its owning module:

| Port | Owner | Methods |
|---|---|---|
| `MessageLog` | `message` | append/read/read_recent/clear_messages |
| `LessonRepo` | `memory` | load/add_lesson, load/save_peer_lessons, list_peers |
| `SurfacedTopicsRepo` | `awareness` | load/save_surfaced_topics |
| `OutboxRepo` | `sync` | load/save_outbox (imports `QueuedEvent` from `crate::sync`) |
| `UserRepo` (+ `UserRecord`) | `routing` | load_users/upsert_user/set_user_last_delivered/delete_users |

**Composite super-trait** — keep one `Arc<dyn Storage>` on `Session`:
`trait Storage: MessageLog + LessonRepo + SurfacedTopicsRepo + OutboxRepo + UserRepo`.
`SqliteStorage` shares one `Mutex<Connection>` across domains — per-port `Arc`s
would fracture that single connection/lock. The composite gives domain-segregated
*interfaces* (each consumer depends on its port — `routing/store.rs` on `UserRepo`,
`sync/builder.rs` on `OutboxRepo`) over one concrete object; the ~30
`session.storage.foo()` call sites and the two `Arc<dyn Storage>` holders
(`session.rs`, `routing/store.rs`) are untouched. Per-port DI is the Phase-2-lift
shape, not now. Method-resolution flows through the super-trait, so the decorator
stays trivial: one `impl<S: Storage> Port for InstrumentedStorage<S>` per port
(its timing log lines survive verbatim).

**`PersistError` relocates to a neutral support home** (beside `LogError` in `log`,
or a tiny `persist` support module), with `pub use` compat aliases left in `sync`
and `board`. Both `sync::OutboxRepo` and `routing::UserRepo` need it; leaving it in
`sync` makes `routing` import `sync`. It is a *generic* persistence error
(`Io | Serde`), not a sync concept — it lives in sync only because the outbox was
its first user. Definition-move only; no signature or call-site change.

**Error stance: do NOT unify.** Every method keeps its exact return type
(`LogError` / `std::io::Error` / `PersistError`). Error unification is out of scope
for a behavior-preserving Phase-0 move. The only type relocation is `PersistError`'s
home (above); everything else is pure trait-declaration relocation.

**`UserRecord` → `routing`, not `roster`.** The sole consumer (`routing/store.rs`)
is a support module; homing the port+entity in the `roster` feature would make
`routing` (support) import a feature — a direction violation. `routing` is where
the port is needed; `UserRecord` has no feature deps (just serde). Whether "User is
a roster entity" is a Phase-1 question once roster/routing are clarified — do not
force it now.

**`GlobalStore` — delete-or-collapse (last).** Zero production wiring, thin
pass-through impls. The quality-correct end is to delete the test-only abstraction
and rely on the free functions (which are the real home and carry their own tests),
or split it mechanically if a reason to keep a global handle emerges. Decide at the
GlobalStore slice; it blocks nothing.

## Slice sequence (each behavior-preserving, dual-backend contract-green)

- **S0 — relocate `PersistError`** to support (`log`/`persist`), compat re-exports
  in `sync` + `board`. ~3 files. Unblocks the two `PersistError`-bearing ports.
- **S1 — prune the dead surface**: delete the six method groups above from the
  `Storage` trait + all three impls; repoint `test_seed` `save_reactions`; trim the
  `contract!` suite. Deletes storage→{comprehension, bookmark, reactions}. This is
  the biggest single coupling win and is pure subtraction.
- **S2 — introduce the composite mechanism + first port** (`awareness::SurfacedTopicsRepo`,
  the smallest): define the port in awareness; `Storage: SurfacedTopicsRepo`; three
  `impl` splits + decorator. Establishes the pattern. ~5 files.
- **S3–S6 — one port per slice**: `message::MessageLog` → `memory::LessonRepo` →
  `sync::OutboxRepo` (rewire fossil `board::orchestrator` → `crate::sync`) →
  `routing::UserRepo` (+ move `UserRecord`). ~5 files each.
- **S7 — GlobalStore**: delete-or-collapse per the decision above.
- **S8 — sweep + fitness**: confirm the `Storage` body declares only neutral infra
  (if any remains, else the composite is bound-only); remove now-dead feature
  `use`s; add the boundary test.

## End-state fitness function

A boundary test (twin of `product_management::boundary_tests`, default-deny style
like `message::boundary_tests`) asserts: `storage/mod.rs` declares **no domain
persistence method** — the composite `Storage`/`GlobalStore` are bound-only
super-traits; feature `use`s appear **only** in the super-trait bound lists (the
blessed edge-composes-features direction), never to name a method's parameter/return
in the trait body; and no repo port imports a storage mechanism (`rusqlite`,
`refinery`, `Connection`, `Path`). The arrow is inverted — features declare the
ports, storage implements them — while `session` still holds one `Arc<dyn Storage>`.

## Out of scope (deliberately)

- Error-type unification (keep exact signatures).
- Per-port DI on `Session` (the Phase-2-lift shape).
- Making bookmark/reactions persistence go *through* the backend (they bypass it
  today via file-path free fns — a separate consistency question, not this move).
- "User is a roster entity" ontology (Phase-1, once roster/routing are clarified).
