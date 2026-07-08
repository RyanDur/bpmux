# RTL + real-backend testing substrate (#1303, Epic #1247 slice 1)

## What this is

A test-fidelity-class that runs the Electron **renderer-only** (React +
jsdom) against a **real bpmux backend** spawned as a subprocess. Sits
between two existing classes:

| Class | Runtime | Backend | Use for |
|---|---|---|---|
| **Unit** (vitest) | jsdom | mock / none | Pure component logic, hooks, reducers |
| **Integration (RTL + real-backend)** | jsdom + bpmux subprocess | real | Renderer ↔ backend wire-format, HTTP+WS round-trips |
| **E2E (Playwright)** | full Electron | real | Native-OS features, cross-process IPC, visual class |

This is **integration-class**, not e2e-class — explicitly. There is no
Electron-main process, no WindowServer, no GPU; cross-process IPC stays
Playwright per #1303 out-of-scope.

## Why it exists

Iteration Aquila surfaced shared-dev-host saturation: Playwright e2e on
bpmux-mac-m1 saturated CPU/GPU/WindowServer under live-dev load,
triggering worker-teardown timeouts. Epic #1247's user-job (CI reliable
+ quota-bypass) requires a testing-substrate that doesn't need shared-
host resources for the bulk of feature-tests. jsdom in-process plus real
bpmux satisfies that constraint while keeping integration-class coverage
that pure mocking can't reach.

Per Ryan-canon 2026-05-12 13:00 UTC: "e2e tests are a way to make sure
we know the capabilities/features of our system. they are a form of
living documentation". RTL substrate **preserves** that property: each
migrated spec retains its capability-statement.

## Version stack

- `vitest` ^4.1.5
- `@testing-library/react` ^16.x (added with #1303)
- `@testing-library/dom` ^10.x (added with #1303; required peer)
- `jsdom` ^29.1.0
- `ws` ^8.x (added with #1303 as WebSocket polyfill for jsdom)
- `@types/ws` (added with #1303)
- `react` ^19.2.1 / `react-dom` ^19.2.1

## How it works

1. **`test/setup/globalSetup.ts`** (vitest globalSetup hook)
   - Spawns one `target/debug/bpmux server` subprocess for the test-run
   - Env: `BPMUX_TEST_MODE=1` (InMemorySourceAdapter, no real GitHub),
     `BPMUX_STORAGE_BACKEND=memory` (#1289), plus the established per-
     test isolation env-vars (BPMUX_AUTH_TOKEN_PATH, BPMUX_DATA_BASE,
     BPMUX_PRIORITY_PATH, BPMUX_USAGE_PATH, BPMUX_BOARD_STORE_PATH,
     BPMUX_BOARD_OUTBOX_PATH) pointed at a tempdir
   - Waits for `GET /api/board` to respond before yielding
   - Reads auth-token from the tempdir file and `provide()`s it + the
     port to tests via vitest's `inject()` channel
   - On teardown: SIGTERMs the subprocess, awaits exit (3s deadline)

2. **`test/setup/setupFile.ts`** (vitest setupFiles, runs per-jsdom-env)
   - Installs a `BrowserCompatWebSocket` polyfill on `globalThis`:
     wraps `ws.WebSocket` to mirror Chromium's silent close-during-
     CONNECTING semantic (production browser swallows the abort
     event; bare ws polyfill emits an error — surfacing as noise
     during renderer-cleanup unless wrapped). Test-layer fix, not a
     renderer-code change.

3. **`test/fixtures/bpmuxShim.ts`** (installable test-only window.bpmux)
   - In production, `window.bpmux` is injected by Electron preload.
   - Tests have no preload; `installBpmuxShim({ wsUrl, authToken, apiBase })`
     sets `window.bpmux` directly. `uninstallBpmuxShim()` restores.

4. **`test/fixtures/renderApp.tsx`** (the test-author surface)
   - `renderApp(opts?)` returns `{ ...RTL-result, cleanup, port,
     authToken, apiBase }`.
   - Defaults read from `inject('bpmuxBackendPort')` and
     `inject('bpmuxAuthToken')`; overrides allowed for negative tests.
   - Installs the shim, calls `render(<App />)`, returns helpers.
   - `cleanup()` unmounts the component then restores the shim.

## When to use this substrate vs Playwright

**Use RTL+real-backend when**:
- Testing renderer ↔ backend wire-format (e.g., StateSnapshot arrives,
  story-list renders, comment-submit round-trips)
- Testing renderer state-machine driven by real backend events
- Wall-clock budget matters (target: <500ms/test per AC8)

**Stay on Playwright when**:
- Cross-process IPC (renderer ↔ Electron-main) is in scope
- Native-OS features (notifications, clipboard, drag)
- Visual-class testing (jest-image-snapshot may join later but defer
  per #1303 out-of-scope)
- True end-to-end including launch + window-manager behavior

## Living-documentation discipline

Each test name in `*.test.tsx` files using this substrate should read
as a **feature-capability-statement**, not a mechanism-checkbox. Per
`feedback_test_fidelity_vocabulary_discipline`: the test-author writes
the spec name as the feature it documents — what bpmux can do — and the
assertions verify that capability holds end-to-end through the real
backend round-trip.

Example (good): `'real backend responds to GET /api/board with 200 (HTTP round-trip)'`
Example (bad): `'fetch returns 200'`

## Migration path for Playwright specs

Slices 2-7 (#1304+) migrate Playwright specs into this substrate
following the slice-1 archetype. Each migration:
1. Read the source Playwright spec; classify as RTL-eligible (no
   cross-process IPC + no native-OS + no visual-class) vs Playwright-
   only.
2. For RTL-eligible: write a new `*.test.tsx` under `src/renderer/src/
   __tests__/` using `renderApp` from `test/fixtures/`.
3. Preserve the spec's capability-statement in the test name.
4. Verify the new test passes + Playwright spec can retire.

## Files added by #1303

- `applications/electron/test/setup/globalSetup.ts`
- `applications/electron/test/setup/setupFile.ts`
- `applications/electron/test/fixtures/bpmuxShim.ts`
- `applications/electron/test/fixtures/renderApp.tsx`
- `applications/electron/test/types.d.ts`
- `applications/electron/src/renderer/src/__tests__/infrastructure.test.tsx` (regression test)
- Updated `applications/electron/vitest.config.ts` (globalSetup + setupFiles)
- Updated `applications/electron/tsconfig.web.json` (include test/)
- Updated `applications/electron/package.json` (devDependencies)
