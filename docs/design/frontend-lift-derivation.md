# Frontend lift — derivation and station log

Derived 2026-07-23 (dev), commissioned by Ryan: "refactor the frontend
and do a lift like we did for the ChosenPicachu project." Reference:
ChosenPicachu's seven-station chain (f4e923c..c5b7093 in that repo),
the App Continuum applied to a React frontend, corrected by this
repo's own store model ("the store IS the engine — slices register
into it").

## The reference chain (ChosenPicachu)

1. `f4e923c` transport and libraries layers per the continuum
2. `8f3b898` bounded context per feature — domain adapters live inside
   the feature that needs them
3. `5556509` pages and test-support layers complete the structure
4. `829cba3` dissolve libraries/ into real owners
5. `d8c367a` the shell folds into pages — pages export routes
6. `daad52c` components receive link targets via context — pages own
   all URLs
7. `c5b7093` curated barrels; orphan specs go home

## The renderer today (applications/electron/src/renderer/src)

158 production files. Features already exist as bounded contexts —
board (69), chat (21), ci, metrics, roles, sidebar, sync — so station
2 of the reference chain is largely done. The live sins:

- **The store lives under `transport/state/`** — store, reducer,
  actions, middleware stack nested inside a door. Upside-down against
  the backend model where the store is the engine at the center and
  doors register into it. An empty `state/` dir already exists.
- **`lib/` is the grab-bag** the reference chain dissolved: hand-rolled
  redux, the result/maybe/asyncResult monads, terminal utilities
  (font, layout, names, api), and per-feature storage hooks
  (useBoardWidthStorage, useTabStorage, useCollapsibleSectionsStorage,
  scopeStorage).
- **`hooks/` is a second grab-bag**: chat hooks (useMentionScoping,
  useMessageFilter, useMessageHistory, useReactions, useTimeline,
  useUnfurl, useNotifications), board hooks (useBoardLayout), terminal
  hooks (useTerminals), roster/presence hooks, shell hooks (useTheme,
  useScrollGutter, useScope).
- **A central `__tests__/`** holds 32 behavior specs orphaned from
  their subjects, plus lint-gate tests and shared fixtures.
- **`api/` and `transport/` split the doors** — REST clients in api/,
  ws + decoders in transport/. Two names for one layer.
- The renderer type mirror (transport/types.ts + stateSyncDecoders
  hand-mirror backend types) is architecture.md's named friction; its
  named fix is codegen, not a move.

## Stations

| # | Station | Reference analog | Status |
|---|---------|------------------|--------|
| 1 | The store comes home: transport/state → state/ — the engine at the center, doors register in | layers (1) | done |
| 2 | Dissolve lib/ into real owners: redux → state/, terminal utils → their feature, storage hooks → their features; the monads keep one small curated home | dissolve (4) | |
| 3 | Dissolve hooks/ into the features that own them | dissolve (4) | |
| 4 | Orphan specs go home; fixtures and lint gates into test-support | specs home (7) | |
| 5 | One door layer: api/ and transport/ reconcile | layers (1) | |
| 6 | Curated barrels where import noise demands | barrels (7) | |

Playbook per station, inherited from the backend lift: byte-identical
moves (git mv), full-text import census before and after, the suite is
the gate (typecheck + vitest + eslint + playwright via pre-push), one
station one commit.

## Parked with Ryan

- The type mirror: single-sourcing backend↔renderer types (ts-rs
  codegen vs schema) is a design conversation, not a station.
- Whether board/'s 69 files want internal structure is intra-feature
  work, out of this lift's scope.
- The shell/pages analog: this is a single-window app; AppShell +
  sidebar sections stand where the reference's pages stood. No station
  unless Ryan wants one.

## Station log

**Station 1 (2026-07-23).** transport/state (store, reducer, actions,
middleware stack, hooks, codecs-facing decode) moved home to state/;
useSelector/useDispatch — store hooks stranded at transport root —
moved with it. state/ reaches outward to transport/ for wire types and
decoders (the door supplies frames; the engine consumes them), never
the reverse for store machinery. 60 importers rewired; byte-identical
moves otherwise. Suite: typecheck clean, 782 vitest green, eslint
clean.
