# Unscheduled container

*Designer — 2026-05-15. Implicit container for loose icebox stories. Sister-shape to named iterations.*

## Why an Unscheduled container exists

Every story in the icebox needs a container-context so drag-affordance is uniform: drag any story TO any container, and the substrate knows where to retag it. An orphan-no-container pattern would create UX-inconsistency (some stories draggable, others not; or drag-into-nowhere semantics).

The Unscheduled container is the implicit "haven't committed to an iteration yet" bucket. It auto-exists for every bpmux-managed repo; users don't create it explicitly.

## Substrate

A canonical iteration-issue with title `Unscheduled` and a sentinel label (e.g., `iteration-unscheduled` or some discriminating tag). Auto-created on first-boot if not already present (idempotent per `feedback_substrate_truth_source_grounding_at_action_time`).

Loose stories carry `iteration-Unscheduled.<index>` labels (e.g., `iteration-Unscheduled.0`, `iteration-Unscheduled.1`) — substrate-isomorphic with named iterations. The renderer treats Unscheduled like any other iteration container, with a few visual signifiers.

Alternative substrate: NO iteration-issue for Unscheduled; just a label-pattern like `priority.<index>` (no iteration prefix) on loose stories. This is simpler-substrate but creates a special case in the renderer + apply_priority_to_board. Designer-lean: iteration-issue shape (uniform substrate) vs special-case. Dev-call.

## Visual signifiers

Distinguish Unscheduled from named iterations to signal "this is the not-yet-committed bucket":

- **Container border**: dashed (vs solid for named iterations)
- **Background**: `var(--bg-subtle)` (no accent-tint)
- **Glyph**: question-mark or hollow-flag (vs filled-flag for committed iterations) — designer-judgment; dev-call on which is faster to render
- **Title**: `Unscheduled` (or `Backlog` — terminology question; see below)
- **Progress bar**: hidden (Unscheduled doesn't have a "completion" semantic)
- **State badge**: hidden (no open/closed state; always implicit-open)
- **Number badge**: hidden (substrate-internal; users don't need to see Unscheduled's issue number)

## Position in icebox

Unscheduled container always sorts AFTER all named iterations (last container in the icebox). Rationale: named iterations are committed work; Unscheduled is the "still being thought about" bucket — natural reading-order is committed-first, then to-be-committed.

Substrate: special-case the sort to always-place-Unscheduled-last regardless of `iteration-order.N` tag (uniform-substrate breaks down at the "always-last" semantic — even with high dense-index, a new iteration could be inserted after Unscheduled). Designer-lean: special-case.

## Default collapse state

- **Open by default** — Unscheduled is where users see what's in the queue to potentially schedule
- Per-user-local collapse-state persists (sister to named iteration)

## Drag-affordance

- Drag a story FROM Unscheduled INTO a named iteration → "scheduling" the story (TagRemoved unscheduled-label; TagAdded named-iteration-label)
- Drag a story FROM a named iteration INTO Unscheduled → "un-scheduling" (TagRemoved named-iteration-label; TagAdded unscheduled-label)
- Drag-within Unscheduled = reorder loose stories (update position-suffix)

## Empty state

When zero loose stories exist:
- Container summary still visible
- Body shows: "All stories are scheduled to iterations." (positive empty-state)
- Drop-target affordance still works — drag any story here to unschedule

## Terminology question

`Unscheduled` vs `Backlog` vs `Inbox` vs `To Triage`:
- `Unscheduled`: descriptive; clearly signals "no iteration assignment"
- `Backlog`: industry-canonical (Scrum/Tracker term); users may have strong associations
- `Inbox`: implies "incoming"; not quite right (these aren't necessarily new)
- `To Triage`: action-implying; might feel pressure-y

Designer-lean: **Unscheduled**. Most descriptive + least loaded. Ryan-call if a different word fits better.

## Cannot be closed

Unscheduled container has no close-iteration affordance:
- Close-iteration button (if any) hidden on Unscheduled
- `close_iteration` MCP tool rejects when given the Unscheduled issue number (error: "Unscheduled cannot be closed; it is the unscheduled-stories bucket")

This is a substrate-level guard; UX-affordance just reflects it.

## Cannot be reordered among iterations

Unscheduled's position is fixed (always last). Drag-iteration-itself affordance is disabled on Unscheduled's summary-row (grip-handle hidden or visibly-disabled).
