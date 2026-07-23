# E2E suite audit — naming, duplication, journeys

Audited 2026-07-23 (dev), commissioned by Ryan: "the test should help
you understand the current intended behaviors, not tell you who asked
you to do them." All 128 specs in applications/electron/tests read in
full by six parallel readers; every claim below is grounded in actual
assertions, not titles.

## Headline

- 128 spec files; 41 carry story numbers in the filename (33
  desk-check-N, 2 dev-smoke-N, 6 others).
- 2 filenames lie about the behavior; several test titles are stale
  or inverted.
- 9 true-duplicate clusters where the same assertion lives in 2-4
  files.
- 12 journey clusters where one user flow is fragmented across 2-7
  files that each re-walk the same setup.
- A handful of tests assert nothing (screenshots + console.log only)
  — green without pinning anything.
- real-backend.spec.ts is a 21-test grab-bag spanning every surface —
  the overlap hub of the suite.

## Rename map (mechanical, behavior-preserving)

| current | behavior-named |
|---|---|
| desk-check-333 | human-board-ready-row-drag-and-keyboard-reorder-persists |
| desk-check-337 | human-drill-in-scroll-reaches-comments-and-post-form |
| desk-check-343 | human-chat-url-unfurl-cards-render-and-open-external |
| desk-check-345 | human-drill-in-comment-optimistic-add-and-rollback |
| desk-check-346 | human-sidebar-derives-groups-and-system-filter |
| desk-check-347-348 | human-chat-avatar-grouping-and-a11y-substrate |
| desk-check-349 | human-chat-filter-pills-persist-and-keyboard-reachable |
| desk-check-350 | human-motion-tokens-respect-reduced-motion |
| desk-check-351 | human-theme-and-board-transitions-reduced-motion |
| desk-check-352 | human-chat-micro-interaction-transitions-token-backed |
| desk-check-353 | human-board-renders-current-iteration-story-on-mount |
| desk-check-354 | human-drill-in-close-exit-transition-reduced-motion |
| desk-check-356 | human-drill-in-comment-cmd-enter-submits |
| desk-check-357 | human-board-ready-row-keyboard-reorder-announces |
| desk-check-361 | human-titlebar-ci-glyph-tracks-conclusion |
| desk-check-366 | human-drill-in-comment-quote-shortcut |
| desk-check-367 | human-drill-in-timeline-major-gates-and-showall |
| desk-check-368 | human-board-acceptance-badge-variants |
| desk-check-370 | human-chat-avatar-tooltip-runtime-role |
| desk-check-371 | human-drill-in-comment-quote-native-menu |
| desk-check-372 | human-titlebar-ci-glyph-ws-push |
| desk-check-375 | human-board-badge-state-aware-tense |
| desk-check-376 | human-board-badge-awaiting-vs-accepted-color |
| desk-check-378 | human-chat-input-kill-routes-to-os-clipboard |
| desk-check-380 | human-chat-inline-mention-pills-scope-shift |
| desk-check-381 | human-chat-inline-markdown-renders-distinct |
| desk-check-382 | human-notification-focus-live-and-api-present |
| desk-check-388 | human-new-messages-pill-absent-at-bottom |
| desk-check-420 | human-chat-per-message-channel-tag |
| desk-check-425 | human-a11y-audit-returns-structured-violations |
| desk-check-443 | human-chat-input-emacs-keybinds |
| desk-check-457-slice5 | human-board-epic-summary-card-and-child-link |
| desk-check-457-slice6 | human-board-epic-state-derived-and-lane-filtered |
| dev-smoke-448 | human-board-column-collapse-toggle |
| dev-smoke-449 | human-board-column-collapse-persists-across-restart |
| human-1170-accept-button-mutates-canonical-via-rest | human-drill-in-accept-button-mutates-canonical |
| human-1720-board-live-transition | human-board-live-transition-no-reload |
| board-default-roadmap-1684 | human-board-defaults-to-roadmap-when-no-iteration |
| close-approach-nudge-1705 | human-board-close-approach-nudge |
| iteration-reorder-persist-1517 | iteration-reorder-persist |
| roadmap-cli-gui-parity-1754 | roadmap-cli-gui-parity |

Lying names (rename regardless of number policy):
- human-observes-accepted-state-after-close-story asserts RETIRED →
  human-observes-retired-state-after-close-story
- human-chat-scope-label-dynamic pins channel routing + connection
  state, not a scope label → human-chat-direct-channel-selection;
  its test 2 title claims a warning renders while asserting its
  absence.
- terminal-session-journey is a single reload+rename case, not yet a
  journey (it becomes one below).
- playwright-in-ci-smoke is a strict CI-config structural gate, not a
  smoke test → ci-workflow-structure-gate.
- Stale titles: desk-check-column's "between IN PROGRESS and
  DELIVERED" asserts Acceptance; titlebar test says "connecting" but
  asserts "reconnecting".

## True duplicates (same assertion, multiple homes)

1. **Emacs input**: desk-check-443 is a strict superset of
   human-chat-input-emacs-cursor-motion + emacs-word-motion (identical
   caret assertions) and overlaps emacs-text-deletion. One file
   should hold the emacs contract.
2. **Ctrl+P**: human-ctrl-p-disambiguation test 1 duplicates
   history-arrow-recall's Ctrl+P; its test 2 duplicates
   keyboard-shortcuts-app-level test 1 nearly verbatim.
3. **CI glyph**: desk-check-361 (conclusions+color) ⊃
   human-ci-glyph-renders-on-titlebar (renders-with-attribute);
   desk-check-372 differs only in delivery path (WS push vs fetch).
4. **Acceptance badges**: desk-check-368/375/376 share near-identical
   drive-to-delivered seeding, splitting variants/tense/color;
   human-story-badge-renders-on-delivered is a subset.
5. **Comment quote**: desk-check-366 (Cmd+E) and 371 (native-menu
   IPC) pin the same blockquote-into-draft contract — two triggers,
   one file's worth.
6. **DM routing**: human-dm-routing-per-role parametrizes exactly
   human-chat-scope-label-dynamic test 1.
7. **Board column headers**: desk-check-column and
   section-count-badge share the (N)/data-empty/header-styling pins.
8. **Board invalidate**: desk-check-353's three tests all reduce to
   "seeded story row visible" — the behavior it gestures at is pinned
   properly by human-board-invalidate-live-refresh.
9. **Column collapse**: dev-smoke-448/449 vs
   human-board-collapse-state-persists-across-reload (toggle+persist
   across restart vs reload) — one disclosure journey's worth.

## Journey clusters (one flow, many files)

1. **Terminal session** (the largest): terminal-session-journey +
   terminals-multi + terminal-rename + terminal-head-close +
   spawning-close-reap + lifecycle-states + close-kill-terminal +
   confirm-before-close + exited-restart-close — spawn → rename →
   multi/switch → reload-restore → exit → close, currently nine
   files re-walking spawn.
2. **Cross-instance WS sync**: all five cross-instance-iteration-*.
3. **Roadmap curation**: outcome-drill-in (spine) + iteration-reorder
   + outcome-reprioritize + outcome-band-move.
4. **Story lifecycle observed**: the four human-observes-* files each
   re-walk the same RPC chain to check one column.
5. **Agent terminal routing**: agent-broadcast-no-self-echo +
   agent-direct-send-by-id + agent-who-discovers-direct-send-target.
6. **CLI story lifecycle**: agent-cli-dev-accepted-badge +
   agent-cli-reject-returns-story-to-ready (shared 40-line preamble).
7. **Drill-in read**: opens-on-story-click + shows-story-body +
   closes-on-escape + comment-input-disabled.
8. **Owner acts on delivered**: owner-action-buttons +
   owner-actions-hidden + reject-form-flow.
9. **Compose-and-send**: enter-sends + addressee-parsing +
   history-arrow-recall + isearch + draft-persists.
10. **Sidebar derivation/scope**: empty-state + direct-roles-derived
    + scope-selection-persists.
11. **Board section disclosure**: section-count-badge +
    section-scroll-and-sticky + collapse-state-persists +
    desk-check-column (+ dev-smoke-448/449).
12. **Motion substrate**: desk-check-350/351/352/354 spread one
    token contract over four surfaces.

## Thin tests (green without pinning)

- desk-check-347-348: three of four tests are screenshots + logs.
- desk-check-343: render and multiple-card tests log without
  asserting.
- desk-check-351/354: several transition props logged, not asserted.
- desk-check-353: all three tests assert only "row visible".

## Split candidate

real-backend.spec.ts holds 21 tests spanning presence (5), reactions
(2), motion, sidebar, titlebar, filters, unfurls, a11y, drag-reorder,
comments — the suite's overlap hub. Split by surface; the "real
unmocked backend" harness is the file's identity, so either split
into real-backend-presence / real-backend-reactions / … or keep as
the one deliberate whole-substrate pass (owner's call).

## Config warts

- SERIAL_HEAVY_SPECS lists a11y-audit.spec.ts, which does not exist.
- Renames touching serial-listed files must update the list.

## Execution phases

1. Mechanical renames (rename map + lying names + config list) —
   behavior-preserving, suite is the proof.
2. Duplicate collapse — per-cluster merges keeping every unique
   assertion; each cluster one commit.
3. Journey consolidation — per-cluster rewrites into one journey
   spec; largest wins first (terminal, cross-instance, observes).
4. Thin-test hardening — every screenshot/log test either gains a
   real assertion or dies.

## Execution ledger (overnight 2026-07-23)

DONE, each through the full gate:
- Phase 1: all 44 provenance/lying names renamed (b6e5b4df); the
  specs' self-referential ATP paths followed — the substantive-gate
  caught the hidden filename coupling exactly as designed.
- RTL + Rust provenance strip (feda68d4): 84 RTL files, 41 Rust
  component files, 24 harness/server files, story-numbered files and
  slice modules renamed.
- Thin-test kill (8d169476): three empty placeholders deleted;
  tautologies deleted or replaced with real pins (the advertised
  send_message warning now pinned against the REAL registry);
  misleading names honest.
- Duplicate collapse (44792577): user_routing facade copies (7),
  bridge share/curate block (9), notification wrappers (3), CiGlyph
  matrix re-pins (2), message-size interior rungs (2).
- Journeys (593413c9 + prior): cross-instance-board-sync (5→1),
  human-observes-story-lifecycle (4→1 with happy/rework/retire
  branches), agent-message-routing (3→1, serial project),
  agent-cli-story-lifecycle (2→1), terminal-session-journey (11→1,
  16 beats); stale a11y glob dropped; SERIAL list updated.
- Typed refusal contracts (0cb1dedc): 88 raw JSON substrings now
  parse structured_error and assert values.
- Final cuts: Ctrl+P duplicate and CI-glyph subset deleted.

BACKLOG COMPLETED (2026-07-23 morning):
- Classicist rewrites: useNotifications drives the real emitter,
  useRefetchOnReconnect dispatches the real store; only the
  Notification global (an OS boundary) stays stubbed.
- Organization: delivery predicate split out of ws_tests; the two
  signal files merged beside their subject; close_story tests have
  their own home; story-labels endpoint test moved to test-support;
  useMessageHistory files swapped to match their subjects; the
  single-file slice_tests wrapper collapsed.
- Seven more journeys: drill-in reading, owner-acts-on-delivered,
  metrics overlay, sidebar channels, compose-and-send, board section
  disclosure, motion substrate (27 files → 7). The merged specs'
  self-referential ATP paths repointed — the substantive-gate caught
  them again.
- Thin-hardening: the evidence-only avatar-grouping spec and the
  board-mount gesture spec deleted (their behaviors are pinned in
  RTL, real-backend, and the invalidate spec); the unfurl log-only
  tests now assert card presence and metadata-less elision; the two
  capture-only motion tests deleted.
- Wiring reads: get_ci_state asserts the conclusion key, the token
  report asserts total_tokens + by_role.

Deliberately kept, with reasons: the sidebar zero-div/span pin (it
IS the owner's html5-semantic canon, sister to the eslint substrate);
useScrollGutter's scrollTo assertion (the DOM call is the observable
boundary in jsdom); the wire-shape golden and boundary-grep fitness
functions.

Residual, low-value tail: the ~250 #NNN comment refs fall to the
comment gate on touch; close_iteration's loose-OR refusal match;
Header CSS-class asserts beside their text equivalents.
