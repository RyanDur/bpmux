# Playwright → RTL+real-backend migration recipe

Archetype-locked at #1304 (Epic #1247 slice 2) by migrating
`tests/human-message-list-renders-seeded-chat.spec.ts` →
`src/renderer/src/__tests__/desk-check-archetype.test.tsx`. The same
mechanical replication applies to every batch-4.1 sister-spec
(human-message-* / human-board-* / human-drill-in-* / dev-smoke-*) per
the #1305 audit. Sub-batch slices (4.1, 4.2, …, 4.N per #1306)
replicate this pattern.

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

## State-leak follow-up

Per-run backend share (vitest globalSetup) means seeded messages
persist across tests in the same run. For slice 4 batches, use unique
seed-markers per test (the archetype uses
`SEED_CHAT_MARKER_BPF13_*`) so assertions remain deterministic
regardless of sibling-test state. A future infrastructure-add could
introduce `/api/test/reset` or per-test backend respawn; not blocking
slice 4.

## Open: visual + native-only specs

Batches 4.2-4.5 (per #1305 audit) need different patterns:
- visual-class specs stay Playwright (no RTL equivalent for
  pixel-diff / screenshot assertions)
- native-OS specs (clipboard / file-pick / IPC) stay Playwright
  unless mock substrate lands

This recipe covers the **batch 4.1 real-backend** class only. Other
class-recipes ship as those batches file.
