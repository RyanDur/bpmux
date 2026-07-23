# RTL + Rust test suite audit

Audited 2026-07-23 (dev), the companion to e2e-suite-audit.md under
the same ruling: tests document intended behavior, never who asked.
Seven parallel readers over 119 RTL files and 250 Rust test files,
findings grounded in assertion bodies. Readers correctly excluded
domain vocabulary (desk_check the workflow stage, falsifier the story
field, slice the store module) from provenance counts.

## Headline

- Provenance naming is the dominant defect and it is enormous: ~80%
  of RTL files open with a story number; Scenario-N / AC-n / SLICE-n /
  falsifier-letter taxonomies are pervasive; two RTL names carry
  wall-clock timestamps; Rust adds ~89 story-suffixed fns in bridge
  alone, story-numbered FILES (story87_acceptance.rs), ac1_/ac2_
  prefixes, sync test modules named slice_7_2/3/4, a
  roundtrip_coherence_/response_correctness_ methodology family (14
  fns), and ~250 #NNN comment references in the integration harness.
  Total: 300+ names to strip.
- Three empty test bodies pass forever (bridge sync_orchestrator,
  integration section_skeleton, hook_auto_fire placeholders); true
  tautologies exist on both sides (membershipContract === 'required';
  mcp_tools asserting its own argument back; ci constants echo).
- ~90 raw JSON-substring assertions in the substrate_enforce family
  (err.contains("\"kind\":\"workflow-role-not-allowed\"")) — one
  serde field-order change from mass breakage; the structured error
  should be parsed and asserted typed.
- Cross-layer duplication is systemic: the delivery/addressing matrix
  is pinned in 5 files across 4 crates with no owning home;
  user_routing's facade tests copy the reducer tests under 7
  identical names; the role-refusal contract is re-pinned 3 layers
  deep per verb across ~18 integration files; RTL pins CM-2 column
  behavior at 3 layers and each state-delta domain at 3-5.
- Classicist canon holds almost everywhere — the exceptions:
  useNotifications (vi.mock of sibling ../state + Notification
  constructor call-counts) and useRefetchOnReconnect (same mock).
  CSS-class-as-behavior is the RTL-wide milder coupling.
- Clean bills: cli/src (fully), server/src (near), protocol
  frame_tests (exemplary), the Rust small-crate long tail, RTL
  pure-function selector/derive files.

## Worst-offender index

RTL:
- board/: 48 of 60 files provenance-tainted; #1339 across 13 files;
  Epic.test.tsx falsifier-a..i names + DOM-structure regexes;
  deriveNowView membershipContract tautology; Timeline/Comments
  "uniqueness" tests that never assert uniqueness; Tabs.test reads
  Tabs.css source text.
- chat/: useNotifications sibling-mock + ctor counts; Sidebar
  #1457 Pictor Scenario-block with canon-cite timestamp; FilterPills
  Scenario 1-9; useNotifications AC2-AC6 names.
- Header.test.tsx renders AppShell — wrong subject; desk-check-
  archetype.test.tsx name/content/page-object all disagree;
  useMessageHistory's named file tests a helper while the hook's
  behavior lives in useMessageHistoryBoundary; useBoardData split
  .ts/.tsx; story-labels-endpoint (pure backend HTTP) lives in board/.
- transport/state: connection.test.ts six story-numbered describes +
  the 11:14-UTC name; state-delta domains pinned decode/apply/hook/
  integration; golden.test.ts overlaps wsDecoders' inline pin (fold
  the inline into the golden); depcruise substrate's live-tree pin is
  a whole-repo gate inside the renderer unit suite.

Rust:
- integration_main_modules: story87/story59 filenames; ac1_/ac2_;
  _1403/_1404/_1548 suffixes; ~250 #NNN comments; role-refusal ×3
  layers; ~90 JSON-substring pins; empty placeholders; is_object()||
  is_array() grade assertions; move/reorder across 6 files.
- bridge: 89 story-suffixed fns + slice/phase tokens +
  15 migrated_path names; share/curate helpers pinned twice (9 tests,
  crate-root tests.rs vs share_message_tests.rs);
  notification push-filter pinned 7x; signal builders split across
  near-identically-named files in two dirs; close_story tests inside
  view_board_basic_tests.rs; crate-root tests.rs is a catch-all.
- transport/routing: ws_tests.rs is a 4-concern grab-bag; delivery
  matrix fragmented (message, ws_tests, chat_filter, agnostic_
  delivery, reduce_tests); identity_tests duplicates reduce_tests
  under 7 identical fn names; roundtrip_coherence_/response_
  correctness_ methodology prefixes; message-size ladder pins the
  same side of one threshold four times.
- product_management + misc: epic_tests 21 story-suffixed fns with
  owner-canon timestamped messages; section_tests 9; sync slice_7
  file + modules; adapter/gate/codec suffixes; order_state_pin
  walks production source grepping for forbidden fields (and signs
  an assertion "(Ryan, 2026-07-07)").

Deliberate and kept (flagged for awareness, not deletion): the
boundary_tests source-grep family are architecture fitness functions
— the renderer's equivalents were replaced by dependency-cruiser;
the Rust ones still guard intra-crate discipline the crate graph
cannot. Sync's hardcoded seam-file list is the brittle one. The
wire-shape json_shape/golden pins are genuine external contracts.

## Program

- U1 Provenance strip (mechanical, ~300+ names): drop story numbers,
  scenario/AC/slice/phase/methodology tokens, timestamps from
  describes, fn names, module names; rename story-numbered files.
  Suite green is the proof. Comment refs (#NNN) fall under the
  no-comments canon where the gate already applies on touch.
- U2 Thin-test kill: delete the three empty placeholders; fix or
  delete the tautologies; harden the misleadingly-named
  (_without_broadcast, Timeline/Comments uniqueness) or delete.
- U3 Duplicate collapse: give delivery/addressing one owning home
  (message crate predicate matrix) with one wiring pin per consumer
  layer; delete identity_tests' seven copies; parametrize the
  role-refusal contract once; dedupe share/curate, notification
  filter, signal files; RTL policy — the pure-fn matrix owns
  behavior, the component keeps ONE wiring test.
- U4 Coupling: substrate_enforce asserts on parsed JSON values;
  useNotifications/useRefetchOnReconnect rewritten classicist;
  CSS-class assertions retargeted at user-visible behavior where one
  exists.
- U5 Homes: split ws_tests by concern; relocate close_story and
  misplaced files; merge the signals pair; rename Header/desk-check-
  archetype to their subjects; move story-labels-endpoint test out of
  the renderer; give the depcruise live-tree pin a tooling home.
