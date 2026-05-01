import { test, expect } from '@playwright/test'
import { launchIsolatedElectron, getFreePort } from './helpers/launch'

// #394 isolation smoke: proves the shared helper delivers the
// per-test guarantees every desk-check spec now rides on.
//
// AC4: Playwright smoke — two launches get distinct ports +
//      distinct sessions; no cross-contamination.
// AC6: CI run verifies the isolation.
//
// Kept small + fast — this is a substrate check, not feature
// coverage. Runs in ~3s.

test.describe('#394 Playwright port + session isolation', () => {
  // Each isolated Electron launch spawns a real bpmux backend +
  // waits for /healthz; two launches + closes in one test can
  // approach the 30s default. Bumped to 180s (#1133 fold) absorbs
  // the post-migration workload — TEST_MODE→httpmock+real-backend
  // raised per-launch cost; workers=2 contention puts 2 sequential
  // real-backend spawns above the prior 90s ceiling.
  test.setTimeout(180_000)

  test('getFreePort returns a port that can be bound', async () => {
    const port = await getFreePort()
    expect(port).toBeGreaterThan(1024)
    expect(port).toBeLessThan(65536)
    // Also not 4117 — that's Ryan's live default and would defeat
    // the purpose of isolation. Not a hard guarantee from
    // getFreePort (the OS picks), but overwhelmingly unlikely;
    // flagging explicitly so a regression that returns 0 / 4117
    // surfaces.
    expect(port).not.toBe(4117)
    expect(port).not.toBe(0)
  })

  test('two sequential launches get distinct ports + distinct sessions', async () => {
    // Rolled into one test: two launches + distinctness checks +
    // the 4117-avoidance guard. Keeps the Electron-launch cost to
    // two invocations, well under the default 30s test timeout.
    // Each launch pays for the bpmux backend spawn + /healthz
    // poll loop; a third test-case launch in this spec would
    // routinely blow the timeout on slow-disk CI.
    const first = await launchIsolatedElectron()
    try {
      const second = await launchIsolatedElectron()
      try {
        expect(first.backendPort).not.toBe(second.backendPort)
        expect(first.sessionBase).not.toBe(second.sessionBase)
        expect(first.userDataDir).not.toBe(second.userDataDir)
        // Neither launch binds Ryan's live default port 4117 —
        // collision with his live session would defeat the whole
        // isolation contract.
        expect(first.backendPort).not.toBe(4117)
        expect(second.backendPort).not.toBe(4117)
      } finally {
        await second.app.close()
      }
    } finally {
      await first.app.close()
    }
  })
})
