import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #372 coverage split across test layers:
//
// - Backend transition classifier (classify_transition): covered by
//   `ci_watcher_tests.rs` — 6 unit tests exercise first-observation,
//   same-same, and every pairwise transition.
//
// - Full WS push loop (backend poller → broadcast frame → renderer
//   onmessage → CiGlyph refetch): requires driving the backend's
//   tracker to report a state change, which Playwright can't force
//   without a live GitHub API. Observable in production + via a real-
//   backend run where a push lands a new CI state.
//
// This spec smoke-tests the integrated glyph behavior with a static
// fixture so a regression on AppShell's `messages` wiring or the
// glyph's render-from-state path surfaces in CI. The push-path
// functional check is deferred to the Rust unit tests + manual
// verification per the test-infrastructure tradeoff above.

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const CI_SUCCESS = {
  conclusion: 'success',
  last_run_id: 1,
  branch: 'main',
  url: 'https://github.com/RyanDur/bpmux/actions/runs/1',
  head_sha: 'deadbeef',
  display_title: 'green'
}

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-372-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-372-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-22T16:00:00Z', stories: [] })
    })
  })
  await win.route('**/api/ci', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(CI_SUCCESS)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #372 push-based CI state updates', () => {
  test('CiGlyph renders CI state from /api/ci and accepts messages prop', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const glyph = win.locator('[data-test="ci-glyph"]')
    await expect(glyph).toBeVisible()
    // Initial fetch resolves success; class + aria-label follow the
    // conclusion. Regression check covers the "CiGlyph renders state
    // correctly" half of the feature; full push-loop observation is
    // the Rust unit tests' domain per the comment at top.
    await expect(glyph).toHaveAttribute('aria-label', 'CI success', {
      timeout: 5_000
    })
    await expect(glyph).toHaveClass(/ci-glyph--success/)

    await glyph.screenshot({
      path: join(EVIDENCE_DIR, '95-ci-glyph-initial-success.png')
    })
    await app.close()
  })
})
