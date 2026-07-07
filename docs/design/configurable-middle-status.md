# Configurable middle — true status

## Morning summary (2026-07-07, overnight build per the owner's directive)

**The configurable middle is real end-to-end, and the falsifier has RUN.** Built overnight
in seven gated commits (2424 nextest + 789 vitest + the 300-spec e2e pre-push suite):

- **No config file → the default three states.** `ready → in progress → acceptance` (plus
  icebox/accepted/retired); one undivided middle; no desk-check; no judges (anyone may
  judge). Pinned by loader tests and a no-config e2e that boots a real server on a bare
  project root.
- **With `.bpmux/workflow.toml` → the board IS the config.** The server lazy-loads the
  project root's config (git-ancestor walk; `BPMUX_PROJECT_ROOT` overrides;
  `BPMUX_WORKFLOW_CONFIG` names a file directly and errors loudly — an explicitly named
  config never silently falls back; a broken file refuses boot). **This repo now carries its
  own `workflow.toml`** declaring the team's dev → desk-check process with designer judging
  desk-check — the team board is unchanged, and the whole test suite dogfoods the file path.
- **Progression is config-defined.** A story moves by a succeed-judgment at its current
  stage; `advance` derives every target from config. Failure lands at the state's `fail_to`
  if configured, else a configured backward transition, else one stage back — and a failure
  at the FIRST middle stage reworks in place (leaving the middle is configurable, never the
  default).
- **Judgment ownership is config-defined and enforced.** `[[state]] judges = [...]` accepts
  role ids or full agent ids; empty/absent means anyone; owner always bypasses. Enforced on
  the new generic verbs `pass_stage` / `fail_stage` (MCP + `bpmux pass-stage` /
  `fail-stage` CLI, full parity). The six named ceremony verbs are untouched — team behavior
  is byte-identical.
- **The frontend renders the configured middle.** The stories state-delta and `/api/board`
  both carry the configured `columns`; the renderer builds its column groups from the wire
  (legacy fallback when absent), buckets stories by stage id, and a three-stage fixture
  renders three middle columns in config order (vitest-pinned).
- **The falsifier:** `configurable_middle_e2e` boots a real server on a temp workflow.toml
  with a three-stage middle (build → review → harden, judges dev/designer/anyone) and drives
  one story end-to-end — configured columns render, a wrong judge is refused BY NAME at each
  guarded stage, a failed review judgment lands back at build, the unjudged stage accepts
  anyone, the walk concludes accepted. Green.

Honest residuals (not silent this time): epic progress rollups (`epicSummary`,
`deriveEpicState`) still assume the two-stage middle when aggregating epic children — a
configured stage falls out of the epic denominator; `useBoardReorder`/close-approach-nudge
deliberately stay on the two-stage compat path (they only touch fixed poles; traced);
`get_story`'s two bundled stage tokens keep their legacy vocabulary ("in progress" with a
space) until the R-4 rename unifies wire vocab; the fixed `Stage` enum still exists at
legacy reader sites but no longer gates any N-stage function. Next in queue: the journey
e2e (icebox→accept, mixed MCP/CLI surfaces), then R-3/R-4.

---

## The original disclosure (2026-07-06), kept for the record

This doc exists because the capability's status lived in dev-private session notes instead
of team-visible space, and the owner operated on the belief that "the middle is
configurable" was a built, working capability. At the time of writing it was not — not
end-to-end. Below is the layer-by-layer truth as it stood THAT MORNING (superseded by the
summary above), the falsifier each claim needed, and a sweep of every other deferral that
was living only in dev-private notes. Rule adopted from this incident: **a deferral
that changes what the product can do is a product decision — disclosed at decision time, in
team-visible space, never only in dev memory. A capability claim stands only after its
falsifier has run.**

## What "configurable middle" means

A team defines its own workflow middle — the stages between Ready and Acceptance — in
`workflow.toml`: one stage, two (the bundled dev → desk-check), or N, with their own names,
role guards, evidence gates, and fail-targets.

## Layer-by-layer status

| Layer | Status | Evidence / falsifier state |
|---|---|---|
| Workflow config + loader (`workflow/config.rs`, `loader.rs`) | **Config-driven, real** | Sections, transitions, roles, gates load from config; `middle_stages_derive_from_the_in_work_section_in_config_order` pins the middle = the in-work section |
| `advance` (verdict → Decision) | **Config-driven, real** | Targets, incl. backward fail-targets, derive from config; no hand-passed targets (verdict-core arc) |
| Board columns (`board_columns`) | **Config-driven, real** | Columns generate from `board_middle_stage_ids` |
| Placement (fine stage) | **Config-shaped, real** | `StageId`-keyed, local (board slice + placements.json), rides events; labels stay coarse (de-leak arc) |
| Domain vocabulary (`Stage` enum: `Dev \| DeskCheck`) | **HARDCODED two stages** | Referenced across ~15 files: `get_story` fine display, `dev_in_flight`, `deliver`, `arrangement` column default, `coalesce`, decode. A third configured stage cannot be named by these readers |
| GitHub decode (`state_labels.rs`) | **HARDCODED** | The desk-check arm matches the bundled stage id; a third stage's label has no decode path |
| Agent surface (MCP tools + CLI verbs) | **HARDCODED six verbs** | `deliver`/`pass`/`fail` are bound to the bundled stages; no verb can succeed/fail at a configured third stage. CLI≡MCP parity exists (`cli/parity_audit.rs`) but over the fixed verb set |
| Display/ceremony tokens (`verdict_as_str`, timeline, reject labels) | **HARDCODED** | Match canonical bundled step ids; byte-identical for the bundled workflow only |
| End-to-end proof | **NEVER RUN** | No test drives a workflow config with a non-two-stage middle through decode → board → verbs → wire. **This is the capability's falsifier and it has never executed.** The claim was unverified the entire time it was load-bearing |

## How this stayed hidden

Two deferral decisions were made by dev and recorded only in dev-private session notes:

1. **Verdict arc:** display readers kept on the fixed vocabulary — "the right display shape
   becomes obvious once the core lands" — flagged there as an *unanswered requirement* for
   the configurable middle, then not surfaced to the team.
2. **Record flip:** codec/readers kept canonical bundled step-id matching — "a config adding
   a middle stage would need a derived default — YAGNI until then."

Both were filed as code-craft (dev's lane). They were product-capability decisions (owner's
lane) wearing craft clothing. The mis-classification is the failure mode the new rule exists
to prevent.

## What completion requires (the remaining arc)

1. Retire the fixed `Stage` enum onto config `StageId`s across all ~15 reader sites.
2. Decode: derive stage-label recognition from the config's middle (with a legacy
   tolerance for the bundled labels).
3. Agent surface: the within-middle verbs generalize — either two engine verbs
   (advance / send-back at the story's current stage, evidence + role from config) or
   config-generated named verbs; section-crossing verbs (`start`, `accept`, `reject`,
   retire) are stable regardless. Shape to be settled in design dialogue, not unilaterally.
4. Display tokens derive from config (bundled names preserved as the default config's
   tokens).
5. **Acceptance = the falsifier finally running:** a real three-stage `workflow.toml`
   driven end-to-end — decode, board columns, placement, verbs, wire, journey e2e — green.

Sequencing is the owner's call. Options on the table: pull it forward (after R-2.3 lands,
before R-3), or after R-4 (R-3/R-4 rework many of the same reader sites; doing both at once
tangles the rename with the retirement — that's the only technical pull toward "after").

## Full deferral sweep (everything else that lived only in dev-private notes)

Product-scope (should have been visible; disclosed now):

- **Per-story pending/sync visibility.** Only a global `SyncStatus.pending_events` count is
  consumed today; there is no per-story "your GH close/push hasn't landed yet" surface. This
  gained weight on 2026-07-06: the R-2.2 one-truth collapse re-homed the accepted-but-GH-open
  divergence signal (the 2026-05-02 false-success class) to "pending outbox entry" — and the
  only user-visible surface of that is the global count. Whether a per-story surface is
  wanted is a product call.
- **`Epic.is_closed` residue.** Epics still carry the stored GitHub-closed mirror
  (`epic.rs`), shown on the roadmap/board wire. Stories shed theirs in R-2 (one truth from
  workflow state); epics were scoped out. The same two-axis divergence class stories had is
  still possible for epics.

Craft-scope (internal structure, no capability effect; listed for a complete sweep):

- Re-homing the six reopen-guards into one state-triggered reconciler (behavior identical
  today; guards live in each verb handler).
- Inline-drain consistency sweep (label/comment/metadata/roster/reorder/closure handlers
  still drain inline; lifecycle handlers are optimistic-async).
- Vestigial `_placement` param threading on `outbound_state_label`/`push_state_transitioned`.
- Renderer tolerates legacy wire states (`ready-for-accept`, `needs-fixes`, `rejected`) and
  keeps a closed-fold branch that derived `gh_state` can no longer reach.

Already team-visible / owner-decided (not silent, listed for completeness): the verdict
plane split (within-middle verdicts local, section crossings shared — Phase B, rides the
persistence arc, Ryan's cut); per-domain file-adapter persistence now with DB as a later
adapter-only swap (Ryan-decided); the board-as-slice reshape doc (labels off the wire,
epics-own-children).
