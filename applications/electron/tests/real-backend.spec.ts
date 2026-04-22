import { test, expect, _electron as electron } from '@playwright/test'

// Real-backend test harness — launches Electron without `win.route`
// stubs, so the renderer exercises the actual network layer:
// REST preflight (CORS), bearer auth, tracker state, state
// derivation. The MVP mocked suite in app.spec.ts catches DOM
// wiring; these tests catch the classes that pass DOM assertions
// but break end-to-end (CORS block, empty buckets on shape
// mismatch, etc.)
//
// Electron's main process spawns `bpmux server` on localhost:4117
// using env vars already present in the test shell. If the backend
// binary isn't built, these tests fail with the spawn dialog error;
// run `cargo build --bin bpmux` from the repo root first.

test.describe('real-backend', () => {
  test('CORS + WS roundtrip: chat region visible, no board error banner when tracker is configured', async () => {
    const app = await electron.launch({ args: ['out/main/index.js'] })
    const win = await app.firstWindow()

    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    // The board fetch is real. When GH_TOKEN + BPMUX_REPO are set,
    // the tracker is configured and the board populates with real
    // data — no error banner. When they aren't set, /api/board 503s
    // and the error banner appears; we skip the strict assertion
    // rather than failing, since CI may run without secrets.
    const hasTracker =
      !!process.env.GH_TOKEN?.length && !!process.env.BPMUX_REPO?.length
    if (hasTracker) {
      // Give the backend a beat to return from the real network.
      // If CORS is broken, this is where the test fails — the error
      // banner appears because fetch() rejected before reading the
      // response body.
      await expect(win.locator('[data-test="board-error"]')).toBeHidden({
        timeout: 10_000
      })
      // And mock-fallback shouldn't kick in — the "mock" badge should
      // be absent when a real response landed.
      await expect(win.locator('.board-mock-flag')).toHaveCount(0)
    } else {
      test.info().annotations.push({
        type: 'skip-strict',
        description:
          'GH_TOKEN/BPMUX_REPO not set; skipping tracker-configured assertions. Board will show MOCK fallback.'
      })
    }

    await app.close()
  })
})
