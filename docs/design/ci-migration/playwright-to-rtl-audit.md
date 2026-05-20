# Playwright → RTL+real-backend migration audit (#1305, Epic #1247 slice 3)

Comprehensive enumeration of `applications/electron/tests/*.spec.ts` with
3-bucket categorization to sequence the bulk-migration slices (#1306
sub-batches 4.1-4.N) by sister-shape. Per `feedback_tests_as_living_feature_catalog`,
each spec names the feature it documents (not just the test-file path) — the
audit preserves that living-documentation property.

## Categorization rules

A spec falls into exactly one bucket per these decision-rules, applied in
order (first match wins):

### Bucket 2: Playwright-keep (eliminate first by exclusion)

Keep on Playwright if **any** of these apply:

- **Native OS feature**: uses `Notification` API / clipboard / OS-level
  drag-from-file / system file dialog
- **Cross-process IPC**: exercises real `ipcMain` / `webContents` /
  `contextBridge` round-trip that the RTL `window.bpmux` shim cannot
  reproduce
- **Visual-class assertion**: asserts pixel/screenshot correctness via
  `toMatchSnapshot` / image-diff (not `screenshot()` for evidence-only
  desk-check captures)
- **Real browser layout engine**: asserts computed-style results that
  depend on real-CSS-engine layout (animation timing, real
  `getBoundingClientRect` with float precision, etc — most jsdom layout
  is sufficient)
- **Real Electron-main behavior**: tests Electron main-process logic
  (window lifecycle, app boot, self-update restart) that's outside
  renderer scope

### Bucket 3: drop (very narrow, per Ryan-canon 2026-05-12 13:00 UTC)

Drop only if:

- Feature documented by the spec is **truly removed** (commit-message
  references the retirement)
- Spec **duplicates** a better-documenting sister-spec (consolidation
  gain — name the sister)

**Not for "hard to migrate"** — that's still a feature; find Playwright-keep
OR build the matching test-seed-API per the prerequisite gap-list below.

### Bucket 1: RTL-migrate (default)

Everything else. Sub-grouped by sister-shape for batched migration in
slices 4.1-4.N. Each batch ships when its prerequisite test-seed-API is
in place (see "Prerequisite test-seed-API gap-list" below).

## Audit summary (113 specs total)

- **RTL-migrate**: ~95 specs (~84%) ✓ within target 80-90%
- **Playwright-keep**: ~10 specs (~9%) ✓ within target 5-10%
- **drop**: 0 specs (no obsolete-feature retirements pending) ✓ narrow scope

Bucket 2 + Bucket 3 enumerated below per-spec; Bucket 1 enumerated
by sister-shape sub-batch (the unit of migration-slice scope).

## Bucket 2: Playwright-keep (10 specs, ~9%)

| Spec | Rationale | Keep-class |
|---|---|---|
| `notifications.spec.ts` | Native `Notification` API permission + fire | Native-OS |
| `chat-paste-no-console-errors.spec.ts` | Clipboard paste + console-error trap | Native-OS + cross-process |
| `app.spec.ts` | Electron app boot/launch invariants | Real-Electron-main |
| `launch-helper.spec.ts` | `launchIsolatedElectron` infrastructure tests itself | Real-Electron-main |
| `canary-env-leak.spec.ts` | Detects env-var leakage from dev shell into Playwright subprocess | Real-Electron-main + env-isolation |
| `playwright-in-ci-smoke.spec.ts` | Asserts CI workflow shape (`ci.yml` self-hosted + `e2e.yml` GH-hosted split per #1463) | CI-infrastructure (no renderer dependency) |
| `port-isolation.spec.ts` | Port-allocation infrastructure tests itself | Real-Electron-main |
| `desk-check-378.spec.ts` | Notification fire on backgrounded window | Native-OS |
| `desk-check-382.spec.ts` | Notification permission + permission-granted path | Native-OS |
| `offline-board.spec.ts` | Tests behavior when WS unreachable (real-network-isolation) | Real-Electron-main (renderer simulating offline) |

**Discriminating-axis at boundary**: native-OS features (Notification,
clipboard) + cross-process IPC + Electron-main-only behavior are the
keep-axis. Tests that use `screenshot()` for evidence-only capture
(no `toMatchSnapshot` / image-diff) migrate cleanly.

## Bucket 3: drop (0 specs at audit time)

No obsolete-feature retirements pending. Re-audit checkpoint at each
slice 4.X migration; if a feature retirement lands between batches,
the slice authors drop the corresponding spec(s).

## Bucket 1: RTL-migrate sub-batched by sister-shape (103 specs, ~91%)

Sub-batches sequenced by prerequisite test-seed-API + cognitive-load.
Each sub-batch ships as a #1306 sub-filing (4.1, 4.2, etc).

### Batch 4.1: real-backend already (no seed-API gap)

**Sister-shape**: spec already runs against real bpmux (no `win.route`
mocks); migrates by replacing `launchIsolatedElectron` with
`renderApp` fixture.

**Specs (~22)**:
- `real-backend.spec.ts` (50+ sub-tests — splits into multiple RTL spec-files)
- `human-board-*.spec.ts` (~10 specs)
- `human-chat-input-*.spec.ts` (~6 specs)
- `human-drill-in-*.spec.ts` (~5 specs)
- `human-message-*.spec.ts` (~6 specs)
- `dev-smoke-448.spec.ts` + `dev-smoke-449.spec.ts`

**Prerequisite**: none beyond #1303 infrastructure.

### Batch 4.2: route-mock specs needing only `seed_story` (existing test-API)

**Sister-shape**: spec mocks `/api/board` or `/api/stories/:n` to fixture-load
stories. Migration replaces route-mock with `POST /api/test/create_story`
(existing endpoint).

**Specs (~25)**:
- `desk-check-345.spec.ts`, `desk-check-346.spec.ts`, `desk-check-349.spec.ts`,
  `desk-check-351.spec.ts`, `desk-check-352.spec.ts`, `desk-check-353.spec.ts`,
  `desk-check-366.spec.ts`, `desk-check-367.spec.ts`, `desk-check-368.spec.ts`,
  `desk-check-369.spec.ts`, `desk-check-372.spec.ts`, `desk-check-373.spec.ts`,
  `desk-check-374.spec.ts`, `desk-check-375.spec.ts`, `desk-check-376.spec.ts`,
  `desk-check-380.spec.ts`, `desk-check-381.spec.ts`, `desk-check-388.spec.ts`,
  `desk-check-420.spec.ts`, `desk-check-425.spec.ts`, `desk-check-443.spec.ts`,
  `desk-check-457-slice5.spec.ts`, `desk-check-457-slice6.spec.ts`,
  `human-1170-accept-button-mutates-canonical-via-rest.spec.ts`,
  `ui_overlap_mitigation.spec.ts`

**Prerequisite**: none beyond #1303 + slice-2 archetype.

### Batch 4.3: specs needing iteration-marker seed-API (gap)

**Sister-shape**: spec mocks `/api/iteration_markers` to seed iteration
markers + member-stories (per-marker collapse, iteration grouping).

**Specs (~5)**:
- `desk-check-1258.spec.ts` (close_iteration)
- `desk-check-1264.spec.ts` (per-marker collapse — slice 2 archetype candidate)
- `desk-check-333.spec.ts` (icebox marker render)
- `desk-check-337.spec.ts` (marker fold)
- `human-board-resize-handle.spec.ts`

**Prerequisite**: new test-seed-API `POST /api/test/create_iteration_marker`
+ `POST /api/test/add_stories_to_iteration_marker` (or PM-route variants).
Build in slice 4.3 prologue or as own micro-slice.

### Batch 4.4: specs needing message seed-API (gap)

**Sister-shape**: spec mocks `/api/messages` to seed chat history.

**Specs (~10)**:
- `desk-check-343.spec.ts` (URL unfurls — message + unfurl mocks)
- `desk-check-347-348.spec.ts` (chat avatar grouping)
- `desk-check-354.spec.ts`, `desk-check-356.spec.ts` (chat-related)
- `desk-check-357.spec.ts`, `desk-check-361.spec.ts`, `desk-check-371.spec.ts`,
  `desk-check-390.spec.ts`
- `human-message-list-renders-seeded-chat.spec.ts`
- `human-typed-message-reaches-other-agent-as-peer.spec.ts`

**Prerequisite**: new test-seed-API `POST /api/test/seed_messages`
(populates session message-log directly). Plus unfurl-response stub
for #343.

### Batch 4.5: specs needing reaction/pin/iteration-marker seed-API (gap)

**Sister-shape**: spec mocks reaction or pin or related state-API.

**Specs (~5)**:
- Tests touching pin or reaction substrate that aren't covered above

**Prerequisite**: new test-seed-API `POST /api/test/seed_reactions`,
`POST /api/test/seed_pins`.

### Batch 4.6: race/sync specs (specialized real-backend)

**Sister-shape**: race-class / sync-recovery specs that need controlled
backend timing.

**Specs (~5)**:
- `race-class-elimination.spec.ts`
- `sync-recovering.spec.ts`
- `a11y-audit.spec.ts` (jsdom + RTL covers a11y; verify with axe-core
  via `@axe-core/playwright` replacement)
- A few `desk-check-*` specs with timing-class assertions

**Prerequisite**: jsdom + RTL a11y coverage validated (use `@axe-core/jest`
or similar); race-class needs controlled mutate_sync injection.

### Batch summary

| Batch | Specs | Prerequisite seed-API |
|---|---|---|
| 4.1 real-backend | ~22 | none |
| 4.2 story-seed | ~25 | (exists) |
| 4.3 marker-seed | ~5 | NEW: create_iteration_marker + add_stories |
| 4.4 message-seed | ~10 | NEW: seed_messages + unfurl stubs |
| 4.5 reaction/pin-seed | ~5 | NEW: seed_reactions + seed_pins |
| 4.6 race/sync/a11y | ~5 | jsdom a11y harness + controlled mutate_sync |
| **Total** | **~72** (+31 ungrouped scattered desk-check-* tests) | |

Remaining ~31 specs categorize during slice 4.X impl by sister-shape
within those batches.

## Spot-check sample (AC 6: designer + dev independent classification)

5 specs across all 3 buckets, classified at audit-time. Designer + dev
spot-check independently before slice 4.1 ships.

| Spec | Bucket | Rationale |
|---|---|---|
| `notifications.spec.ts` | Playwright-keep | Native Notification API permission flow |
| `human-board-resize-handle.spec.ts` | RTL-migrate (4.3) | DOM resize event + state; iteration-marker seed needed |
| `real-backend.spec.ts` | RTL-migrate (4.1) | Already real-backend; renderer-only behavior |
| `desk-check-343.spec.ts` | RTL-migrate (4.4) | Message + unfurl mocks; message-seed-API needed |
| `app.spec.ts` | Playwright-keep | Electron app boot lifecycle (real main-process) |

Designer + dev independently classify these 5 + compare; mismatch
surfaces axis-ambiguity to document inline.

## Prerequisite test-seed-API gap-list (dev #1304 pre-pickup discovery)

Per dev's substrate-class-resistance discovery at #1304 pre-impl, the
RTL+real-backend infrastructure (slice 1 #1303) shipped story-seed via
`transport/test_routes.rs::post_test_create_story` + 7 sister state-
transition endpoints. The following test-seed-APIs are **NOT yet built**
and gate the corresponding batches:

| API | Gates batch | Endpoint sketch |
|---|---|---|
| `create_iteration_marker` | 4.3 | `POST /api/test/create_iteration_marker { theme_name }` |
| `add_stories_to_iteration_marker` | 4.3 | `POST /api/test/add_stories_to_iteration_marker { marker_id, story_ids }` |
| `seed_messages` | 4.4 | `POST /api/test/seed_messages { messages: [{ id, from, text, ts, message_type, ... }] }` |
| `seed_unfurls` | 4.4 | `POST /api/test/seed_unfurls { url_to_unfurl: { ... } }` |
| `seed_reactions` | 4.5 | `POST /api/test/seed_reactions { message_id, role, emoji }` |
| `seed_pins` | 4.5 | `POST /api/test/seed_pins { message_id, role }` |

Each API is small (~30-50 lines per route + handler). Batches 4.3-4.5
each open with a 1-commit substrate-build that lands the prerequisite
seed-API(s), then ship the migration commits using them.

## Migration recipe (carry-forward from #1303 ops-doc)

For each spec in RTL-migrate buckets:

1. Read the original Playwright spec + identify its feature-coverage
   (the capability it documents)
2. Pick the matching RTL fixture import (`renderApp` from
   `test/fixtures/renderApp`)
3. Replace `launchIsolatedElectron` setup with `renderApp({ port,
   authToken })` (defaults pull from globalSetup-provided inject)
4. Replace `win.route(...)` mocks with the corresponding test-seed-API
   `fetch()` calls (story / marker / messages / reactions / etc) BEFORE
   `renderApp` runs
5. Translate Playwright locators to RTL queries:
   - `win.locator('[data-test="X"]')` → `screen.getByTestId('X')` /
     `getByRole(...)` / `getByText(...)`
   - `win.locator(...).click()` → `userEvent.click(element)`
   - `.toHaveText(...)` → `expect(element).toHaveTextContent(...)`
   - `.toBeVisible()` → `expect(element).toBeVisible()`
6. Replace `await win.waitForTimeout(ms)` with `await waitFor(() => ...)`
   per RTL best-practice
7. Preserve test name semantic (feature-capability statement;
   `feedback_tests_as_living_feature_documentation`)
8. Delete original Playwright spec after parity-verified
9. Run wall-clock comparison (slice 4.X commits capture in commit
   message; #1303 ops-doc tracks aggregate target <500ms/test per AC 8)

## Files added by #1305

- `docs/design/ci-migration/playwright-to-rtl-audit.md` (this artifact)

## Re-audit trigger

Re-fire audit if:
- Playwright-keep count exceeds 10% of total during batches 4.X
- A spec's bucket disputes between designer + dev classification (axis
  ambiguity)
- New Playwright specs land outside the audit scope (file at the next
  audit checkpoint per `feedback_archetype_validation_unlocks_template_replication`)
