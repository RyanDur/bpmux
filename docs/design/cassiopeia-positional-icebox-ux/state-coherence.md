# State-coherence UX

*Designer — 2026-05-15. Optimistic-update + conflict-resolution + cross-instance state-delta visual considerations for the canonical-via-GH label substrate.*

## The substrate axis

Iteration membership + position lives as labels on GitHub issues (`iteration-<ThemeName>.<index>` on member stories — e.g., `iteration-Cassiopeia.0`, `iteration-Cassiopeia.1`; `iteration-order.<index>` on iteration-issues). Canonical-via-GH; the same substrate axis bpmux uses for state, parent-epic, and other story metadata. Theme-name in label is space-to-hyphen-mapped if the iteration title contains spaces (canonical convention). Position is dense-index (0, 1, 2, ...). Renumber-cascade on insert is bounded by step 10's outbox last-update-wins coalesce (per `(issue, label-class)` key — supersedes intermediate sequences within the drain window).

Cross-instance propagation uses bpmux's existing `BoardEvent::TagAdded` / `BoardEvent::TagRemoved` events broadcast over WebSocket (sub-second cross-instance fan-out). External edits (label changes from outside bpmux, e.g., GH web UI) are caught by the existing 30s GH-poll. No new infrastructure required.

## Optimistic-update behavior

When the user drags a row + drops:

1. **Immediate** — local board view reflects the new position; cross-container drag also visually relocates the row
2. **Async** — canonical-mutation-chain fires:
   - For within-container drag: TagRemoved(`iteration-<ThemeName>.<OLD-position>`) + TagAdded(`iteration-<ThemeName>.<NEW-position>`)
   - For cross-container drag: TagRemoved(`iteration-<OldTheme>.<OLD-position>`) + TagAdded(`iteration-<NewTheme>.<NEW-position>`)
   - For iteration-reorder: TagRemoved on iteration-issue + TagAdded with new order-position
3. **On success** — state-delta broadcasts to other instances; local view confirms (no visual change)
4. **On failure** — roll back to pre-drag state; surface visible error indicator on affected row

### Visible-error treatment on rollback

When canonical-via-GH write fails (network down, GH rate-limit, label-conflict):
- Row animates back to pre-drag position
- Brief banner above the icebox: "Couldn't save change: <reason>. Retry?" with a `Retry` button
- Banner persists until: user clicks Retry + write succeeds, user dismisses, OR any successful canonical-write clears it
- Other interactions in the icebox stay live (don't lock the whole list)

## Conflict-resolution

Two instances reorder the same iteration concurrently (e.g., both retag the same story with different position-suffixes). Resolution: **last-write-wins per GH label-edit timestamp.**

When the losing instance's TagAdded conflicts with an existing label-state on GH:
- GH-API returns the latest label-set (post-other-instance's write)
- bpmux's renderer rolls back the optimistic local-state to match GH-canonical
- Visible banner: "Another bpmux user reordered first. Your change rolled back." with a `Try again` button (re-fire the drag-intent against the now-current state)

Rationale: last-write-wins matches bpmux's existing canonical-via-GH semantics for other mutations (label-edits, comments, etc.). Merge-shape (CRDT-positional-ordering) is significantly more complex; out-of-scope for v1.

## Cross-instance state-delta visual

When instance A's drag persists + state-delta broadcasts to instance B:
- Instance B's affected container(s) re-render with the new order
- Brief slide-animation (300ms ease-out) on the row(s) that moved positions
- For cross-container moves: row fades out of source container + fades into receiving container (200ms each, sequential)
- No banner or notification — the visual change IS the signal

When state-delta indicates a NEW iteration was created in another instance:
- The new iteration's container fades-in at its canonical order-position (200ms fade)
- Empty body shows the "drag stories here" affordance (per [iteration-container.md](iteration-container.md) empty-state)

When state-delta indicates an iteration was CLOSED in another instance:
- The container dims to closed-state visual (per [iteration-container.md](iteration-container.md) closed-state)
- Members visible inside the body also dim (frozen state)
- Open/expanded `<details>` may auto-collapse to closed-default (per-user-judgment; default: keep open if user had it open, just dim)

## Loading + stale states

Initial canonical-via-GH fetch in-flight:
- Icebox shows existing skeleton-rows pattern (`<li class="board-story-row board-story-row--skeleton">`)
- Iteration containers render as skeleton-summary rows with skeleton body

If canonical-via-GH fetch fails entirely (GH API down):
- Icebox shows last-known-good state (from in-memory cache OR a stale-marker badge)
- Banner: "Couldn't reach GitHub. Order may be out-of-date." with `Retry` button
- All drag/drop is DISABLED (graceful degradation; mutations can't persist)

## Migration UX (one-shot at substrate-pivot)

When the substrate-pivot from `priority.json` to label-canonical fires (#1301 Slice D rework):
- Boot-time auto-heal runs the migration in-place
- For each iceboxed story: derive the destination iteration's theme-name + position-suffix from old `priority.json` state; emit TagAdded(`iteration-<ThemeName>.<position>`)
- For un-anchored stories: TagAdded(`iteration-Unscheduled.<position>`) per the Unscheduled container substrate
- For each iteration-issue: emit TagAdded(`iteration-order.<position>`) to assign iteration-level order
- Audit-log committed: `docs/audit-logs/2026-05-NN-cassiopeia-label-substrate-migration.md` enumerates per-story per-iteration label-mutations + the migration-derived order
- After migration succeeds: `priority.json` icebox-state retired (other priority.json columns may persist for non-icebox state if needed)
- Idempotent: re-runs after first pass are no-ops (sister to existing migration pattern at `migrate_icebox_unified`)

## Out of scope

- Real-time multi-cursor (showing other users' drag-in-progress)
- Undo/redo stack for icebox drags
- Per-user order overrides (everyone sees the same canonical order)
- Offline-mutation queue (mutations require live GH-API access; offline = graceful degradation)
