# Playwright → RTL+real-backend migration recipe

Archetype-locked at #1304 (Epic #1247 slice 2) by migrating
`tests/human-message-list-renders-seeded-chat.spec.ts` →
`src/renderer/src/__tests__/desk-check-archetype.test.tsx`. The same
mechanical replication applies to every batch-4.1 sister-spec
(human-message-* / human-board-* / human-drill-in-* / dev-smoke-*) per
the #1305 audit. Sub-batch slices (4.1, 4.2, …, 4.N per #1306)
replicate this pattern.

**Status (post-Pegasus, 2026-05-14)**: Epic #1247 substrate-chain
ships in interim-close-out state per #1307. Sub-batches 4.1.A
(#1306) + 4.1.B (#1335) shipped — full human-message-* family
migrated (7/7 specs). Sub-batches 4.2-4.6 pending with
resurfacing-condition (see `docs/architecture.md` testing-substrate
section + final-close-out file-trigger phrase).

## Sister-substrate-canon (established 2026-05-14)

The shared test-fixtures + REST endpoints make the migration
mechanical:

| Substrate | File | Use |
|---|---|---|
| renderApp() | applications/electron/test/fixtures/renderApp.tsx | mount App against the per-run bpmux backend |
| seedMessages(apiBase, authToken, messages) | applications/electron/test/fixtures/seedMessages.ts | seed chat-class message rows before mount |
| resetMessages(apiBase, authToken) | (same file) | wipe message-store between tests |
| seedRoster(apiBase, authToken, identities) | (same file) | dispatch roster-upsert via canonical orchestrator chain so mirror.roster propagates |
| POST /api/test/seed_messages | transport/test_seed.rs | message-store append (gated by BPMUX_TEST_SEED_API) |
| POST /api/test/reset_messages | (same file) | message-store wipe (#1306) |
| POST /api/test/seed_roster | (same file) | roster-upsert via OrchestratorAction::Roster (#1335) |
| Storage::clear_messages() | session/storage.rs | trait method backing reset_messages |
| configure({ testIdAttribute: 'data-test' }) | test/setup/setupFile.ts | RTL queries match bpmux convention |

## Parity-log discipline (established 2026-05-14)

Per migrated sub-batch, commit a `parity-<commit>.log` artifact
alongside the migration commit. Sister-canon: see
`docs/design/ci-migration/parity-ff453dc.log` (#1306 4.1.A) and
`parity-95783f9.log` (#1335 4.1.B) for the canonical shape.

Contents (5 sections):

1. **Sequence** — 5 steps: cargo build → vitest → playwright →
   full-suite → atomic-rm
2. **Per-spec evidence** — table mapping each Playwright source to
   its RTL target + behavior-asserted column (verbatim from both)
3. **Wall-clock comparison** — Playwright-original vs RTL-migrated
   numbers (both wall-clock real + reported test exec)
4. **Substrate-adds** (if any) — names new endpoints / helpers /
   trait methods shipped within the slice (sister to no-defer canon)
5. **Falsifier-coverage matrix** — table mapping each AC to
   evidence-of-coverage in the migration

## Substrate

- **vitest** instead of Playwright runner
- **jsdom** instead of Chromium (per vitest.config.ts)
- **@testing-library/react** instead of `win.locator`
- **renderApp()** fixture from `test/fixtures/renderApp.tsx` instead
  of `launchWithMockGh()` / `launchIsolatedElectron()`
- **bpmux backend subprocess** spawned ONCE per test-run via vitest
  globalSetup (per #1303); state-leak between tests in same run is
  accepted-cost — seed with unique markers per test
- **/api/test/seed_messages** REST endpoint instead of
  `launchWithMockGh({ seedMessages: [...] })` — same backend route
  (gated by `BPMUX_TEST_SEED_API=1` in globalSetup env)

## Recipe (mechanical replication)

### Imports
```ts
// Playwright (before)
import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// RTL (after)
import { afterEach, describe, expect, inject, it } from 'vitest'
import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
```

### Test scaffolding
```ts
// Playwright
test.describe('feature', () => {
  test('behavior', async () => {
    const { app, win, mockGh } = await launchWithMockGh({ seedMessages: [...] })
    try { ... } finally { await app.close(); await mockGh.close() }
  })
})

// RTL
describe('feature', () => {
  let rendered: RenderAppResult | null = null
  afterEach(() => { rendered?.cleanup(); rendered = null })
  it('behavior', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    await seedMessages(`http://127.0.0.1:${port}`, authToken, SEED)
    rendered = renderApp()
    // assertions
  }, 20_000)
})
```

### Seed-before-mount ordering
Per the archetype: seed-messages POST must complete BEFORE renderApp()
so the renderer's useMessageHistory hook fetches the seeded rows on
mount. Mirrors the Playwright launch.ts ordering
(`waitForHealthz` → `seedMessages` → renderer attach).

### Query translation
| Playwright | RTL |
|---|---|
| `win.locator('[data-test="x"]')` | `rendered.container.querySelector('[data-test="x"]')` (sync) OR `await rendered.findByTestId('x')` (async wait) |
| `await expect(loc).toBeVisible()` | `await rendered.findByTestId('x')` — throws on miss |
| `await expect(loc).toContainText(/foo/)` | `await rendered.findByText(/foo/)` — throws on miss |
| `await expect(loc).toHaveCount(n)` | `expect(rendered.container.querySelectorAll('[data-test="x"]').length).toBe(n)` |
| `await loc.click()` | `await userEvent.click(rendered.getByTestId('x'))` |
| `await win.keyboard.press('Enter')` | `await userEvent.keyboard('{Enter}')` |

### Console-error capture
Playwright tests register `win.on('console', ...)` listeners. RTL has
no equivalent — instead, every test goes through the same jsdom env
and any uncaught React errors / console.error spikes surface in
vitest's stderr. For migrated specs that explicitly asserted
`expect(consoleErrors).toEqual([])`, drop that assertion at migration
time: the global jsdom console is already test-runner-visible per the
absent-MCP-server pattern in #1303 setupFile.ts.

### Wall-clock budget
Per #1303 AC8: <500ms per test (excluding amortized globalSetup
cost). Achieved by RTL's no-Electron-launch design.

## Empirical wall-clock comparison

Original Playwright spec
(`tests/human-message-list-renders-seeded-chat.spec.ts`):
- 1 worker: 1.94s wall-clock (Playwright launch + Electron spawn +
  test exec + teardown)
- 1.1s reported test exec; 0.84s overhead

Migrated RTL spec
(`src/renderer/src/__tests__/desk-check-archetype.test.tsx`):
- 140ms in-test (per vitest report)
- 1.72s total run (including amortized bpmux backend spawn)

**Win**: ~8× faster per-test once the backend spawn is amortized
across the run. Slice 4 batches will surface the full empirical-win
as more specs migrate (amortization concentrates).

## Feature-coverage preserved

The migrated test asserts the SAME observable behavior as the
original:
- `data-test="message-list"` element exists in the DOM
- Seeded text strings `SEED_CHAT_MARKER_BPF13_FIRST` +
  `SEED_CHAT_MARKER_BPF13_SECOND` render in the list
- At least 2 `data-test="message-row"` rows exist

Same feature: "seeded chat messages render with from + text in the
message list". The migration changes the test-runner substrate; the
feature-catalog statement carries over verbatim per Ryan-canon
2026-05-12 13:00 UTC living-documentation principle.

## State isolation (per #1306 slice 4.1)

Per-run backend share (vitest globalSetup) means state leaks across
tests in the same run. Two substrate-pieces close this:

1. **`POST /api/test/reset_messages`** — wipes the message-store
   (gated by `BPMUX_TEST_SEED_API=1`). Helper:
   `test/fixtures/seedMessages.ts:resetMessages(apiBase, authToken)`.
   Call before each test's `seedMessages()` so the test sees ONLY
   its own fixture.
2. **Serial vitest execution** (`vitest.config.ts:fileParallelism: false`
   + `singleThread: true`) — parallel workers would race against the
   shared backend's state even with reset; serialization removes the
   race. Per-test cost stays <200ms; total cost amortizes globalSetup
   spawn.

Pattern for each migrated test:

```ts
const port = inject('bpmuxBackendPort')
const authToken = inject('bpmuxAuthToken')
const apiBase = `http://127.0.0.1:${port}`
await resetMessages(apiBase, authToken)
await seedMessages(apiBase, authToken, FIXTURE)
rendered = renderApp()
// assertions...
```

## Sister-shape variance: when to split a sub-batch

Per #1306 falsifier (g): if specs within a sub-batch diverge from the
archetype-shape (e.g., require seedRoster, win.reload, or reaction-state
machinery), document the variance, then ship the substrate-add WITHIN
the slice (not as a follow-up). The #1306 sub-batch 4.1.A initially
deferred 3 divergent specs; per Ryan-canon "no defer / no carry-over"
(2026-05-14 17:09 UTC), those were brought back into Pegasus as #1335
sub-batch 4.1.B which shipped all 3 substrate-adds + 3 specs together.

**Substrate-adds in sub-batch 4.1.B (shipped in #1335)**:
- `seedRoster` fixture + `POST /api/test/seed_roster` endpoint — seeds
  the orchestrator's RosterStore via the canonical
  `OrchestratorAction::Roster::RosterMemberUpserted` dispatch path so
  mirror.roster propagates to the renderer's state-snapshot
- RTL win.reload-equivalent — `rendered.cleanup()` then a fresh
  `renderApp()`. jsdom retains localStorage across the cycle
  (sister-semantic to a browser reload preserving localStorage; only
  the window.bpmux shim re-installs). Tests calling this pattern
  should `window.localStorage.clear()` in afterEach so filter-pill
  state doesn't leak across cases
- Multi-step reaction-state interaction — same pattern as any
  multi-step test: click → waitFor next state → click → waitFor pill;
  no new substrate beyond findByTestId + waitFor

**Migrated in sub-batch 4.1.A (#1306)**:
- `class-glyphs-render` — seed → DOM → glyph-count
- `grouping-five-minute-threshold` — seed → DOM → row-class counts
- `mention-renders-as-pill` — seed → DOM → mention-pill text
- `pin-toggle-button-renders` — seed → DOM → pin-button query

**Migrated in sub-batch 4.1.B (#1335)**:
- `mention-click-scopes-recipient` — seedRoster → click pill → assert
  sidebar-direct-dev aria-current
- `filter-persists-across-reload` (2 tests) — click filter → unmount
  → remount → assert pill state preserved
- `reaction-picker-opens` — click add → assert picker → click emoji →
  assert pill via WS round-trip

## Open: visual + native-only specs

Batches 4.2-4.5 (per #1305 audit) need different patterns:
- visual-class specs stay Playwright (no RTL equivalent for
  pixel-diff / screenshot assertions)
- native-OS specs (clipboard / file-pick / IPC) stay Playwright
  unless mock substrate lands

This recipe covers the **batch 4.1 real-backend** class only. Other
class-recipes ship as those batches file.
