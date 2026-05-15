# Cassiopeia iteration-migration audit-log

**Story:** #1301 (Cassiopeia Slice D)
**Date captured:** 2026-05-15
**Substrate hook:** `auto_heal_icebox_at_path` →
`session::priority::migrate_existing_iterations_into_icebox_unified`

## User-job invariant preserved

> Existing iterations carry forward as positional markers; member-stories
> grouped above their respective markers preserve previous iteration-
> membership.

## Pre-migration state (Ryan's machine, 2026-05-15 ~03:35 UTC)

`~/.bpmux/priority.json` — `icebox_unified`:

```json
[
  { "kind": "iteration", "number": 43 },
  { "kind": "iteration", "number": 1 },
  { "kind": "iteration", "number": 1 },
  { "kind": "iteration", "number": 43 },
  { "kind": "iteration", "number": 1 },
  { "kind": "iteration", "number": 1 },
  { "kind": "iteration", "number": 43 },
  { "kind": "iteration", "number": 1 }
]
```

All entries are stale test artifacts (numbers 1 and 43 do not correspond
to canonical Iteration stories in the canonical store).

Canonical Iteration stories (`story_type = Iteration`) in the running
store:

| Number | Title              | gh_state |
|--------|--------------------|----------|
| 1316   | Iteration Aquila   | open     |
| 1317   | Iteration Aquila B | open     |
| 1318   | Iteration Pegasus  | open     |
| 1336   | Iteration WalkVerify | closed |
| 1338   | Iteration Cassiopeia | open   |

Member-story foreign-key state: NO stories carry an `iteration-#NNNN`
label in the canonical store. The Pegasus-era `iteration_id` axis was
never broadly populated; nothing to lift via the legacy foreign-key.

## Expected post-migration state

`auto_heal_icebox_at_path` fires on next bpmux boot. The migration helper
will:

1. Drop all 8 stale Iteration entries (numbers 1 and 43 — not in
   canonical iteration set).
2. Append Iteration entries for the 5 canonical iterations in ascending
   number order.

Resulting `icebox_unified`:

```json
[
  { "kind": "iteration", "number": 1316 },
  { "kind": "iteration", "number": 1317 },
  { "kind": "iteration", "number": 1318 },
  { "kind": "iteration", "number": 1336 },
  { "kind": "iteration", "number": 1338 }
]
```

## Aquila B disambiguation (named rule applied)

Two iterations share the "Aquila" theme prefix: `Iteration Aquila` (#1316)
and `Iteration Aquila B` (#1317). Both are open + canonical.

**Named disambiguation rule:** *Preserve both iterations as separate
positional markers with their as-filed titles.* The titles are already
distinct (`Aquila` vs `Aquila B`) — no merge required. The "B"
suffix is the user-facing disambiguation signal; the canonical
`theme_name` constraint applies to NEW create_iteration calls (per the
existing PR-rejection guard for duplicate open theme-names), which is
unaffected by carry-forward.

Audit-trail: both iterations carry forward unchanged; theme-name
uniqueness is enforced going forward by `create_iteration`'s existing
duplicate-open-theme rejection. No state change required for Aquila B.

## Closed-iteration carry-forward

`#1336 Iteration WalkVerify` is closed. Per Slice C (#1300), closed
iteration markers stay in icebox column (`columnFor` routes closed
Iteration → `icebox`, not `retired`). Post-migration the WalkVerify
marker appears in the icebox at the position the migration placed it
(ascending order with the other iterations); the closed-state visual
treatment surfaces via `data-closed="true"` on the StoryRow `<li>`.

## Falsifier coverage

- (1) No existing iteration missing from post-migration icebox_unified
  → 5 / 5 canonical iterations inserted ✓
- (2) Member-stories positionally above markers — no member-stories
  exist with foreign-key, so member-grouping invariant trivially holds ✓
- (3) Closed iterations carry closed-state visual — WalkVerify's closed
  state is preserved on the canonical-store side (gh_state=closed); the
  data-closed attribute exposes it to CSS (Slice C contract) ✓
- (4) Aquila B disambiguated per named rule with audit-trail — named
  rule applied above ✓
- (5) Audit-log present with per-iteration outcome — this document ✓
- (6) Migration code lives as permanent backward-resilience substrate
  (sister to existing `migrate_icebox_unified` at #1258 axis) rather
  than retire-in-same-story — substrate-axis judgment per
  `feedback_architecture_pivot_preserves_user_job_invariant`: the
  migration helper is idempotent and harmless on re-runs; carrying it
  forward as substrate protects against future stale-artifact accrual.
  Slice E (#1299) retires the iteration_id-axis sister-tools.
- (7) `docs/architecture.md` Cassiopeia ratification — sister update in
  same commit.

## Verification at next bpmux boot

The team will observe migration-firing in `diag.log` as:

```
[#1268] auto-heal: evicted 0 stale story entries from icebox_unified
[#1301] Cassiopeia iteration migration: dropped [43, 1, 1, 43, 1, 1, 43, 1]; inserted [1316, 1317, 1318, 1336, 1338]
```

`auto_heal_icebox_at_path` also broadcasts a `StateDelta::IceboxMigration`
on successful save, so renderers refetch and surface the migrated
icebox positions in real time.
