import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'

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
//
// Session isolation: each test gets a fresh BPMUX_SESSION_BASE
// under the OS temp dir so sent messages + state don't leak into
// the user's live ~/.bpmux log. Without this, a test that exercises
// the send path writes into the dev's real chat history (the
// 2026-04-22 seed-message-leak-into-team-chat incident).
async function launchWithIsolatedSession(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
  sessionBase: string
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-playwright-'))
  const app = await electron.launch({
    args: ['out/main/index.js'],
    env: { ...process.env, BPMUX_SESSION_BASE: sessionBase }
  })
  return { app, sessionBase }
}

test.describe('real-backend', () => {
  test('CORS + WS roundtrip: chat region visible, no board error banner when tracker is configured', async () => {
    const { app } = await launchWithIsolatedSession()
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

  // #347 a11y substrate — real-backend assertions. Tracker-independent:
  // chat-region role, sidebar chip font-size, and i-search failing
  // glyph are all renderer-intrinsic, so this test runs in CI whether
  // or not GH_TOKEN is configured.
  test('a11y substrate: chat role=log, sidebar chip ≥12px, i-search failing ✗ glyph', async () => {
    const { app } = await launchWithIsolatedSession()
    const win = await app.firstWindow()

    // AC9(a): chat-region is a live log region.
    const chatRegion = win.locator('[data-test="chat-region"]')
    await expect(chatRegion).toBeVisible()
    const messageList = win.locator('[data-test="message-list"]')
    await expect(messageList).toHaveAttribute('role', 'log')
    await expect(messageList).toHaveAttribute('aria-live', 'polite')

    // AC9(b): every sidebar channel chip computes to ≥12px. The chip
    // inherits font-size from `.sidebar` (13px / 0.8125rem); if any
    // future override dips below 12px, this catches it. Reading
    // getComputedStyle for authoritative browser-layer value rather
    // than asserting a specific class/attribute, so the test survives
    // styling refactors as long as the observable outcome (text large
    // enough to read) holds.
    const chips = win.locator('[data-test="sidebar"] .sidebar-item')
    const chipCount = await chips.count()
    expect(chipCount).toBeGreaterThan(0)
    for (let i = 0; i < chipCount; i++) {
      const pxString = await chips.nth(i).evaluate(
        (el) => window.getComputedStyle(el as Element).fontSize
      )
      const px = parseFloat(pxString)
      expect(px, `sidebar chip #${i} font-size (${pxString})`).toBeGreaterThanOrEqual(12)
    }

    // AC9(c): i-search failing state contains the ✗ glyph. Ctrl+R
    // opens the strip regardless of history state (post-347 product
    // behavior), so the test stays self-contained — no need to seed
    // a sent message first. A uuid-tagged query guarantees no match.
    const textarea = win.locator('[data-test="chat-input-textarea"]')
    await textarea.focus()
    await textarea.press('Control+r')
    const noMatchQuery = 'zzznevermatchesxyzzy-' + Date.now()
    await win.keyboard.type(noMatchQuery)
    const isearchStrip = win.locator('[data-test="chat-input-isearch"]')
    await expect(isearchStrip).toBeVisible({ timeout: 3_000 })
    await expect(isearchStrip).toContainText('✗')

    await app.close()
  })

  // #348 visual differentiation — grouping + Ryan-shape avatar.
  //
  // This test narrowly mocks `/api/messages` (hydration payload) via
  // win.route to seed a deterministic fixture. That's a scoped
  // fixture for a rendering-logic test (given messages X, what does
  // the DOM render?) — CORS and network substrate are exercised by
  // the earlier real-backend tests, so this narrow mock doesn't
  // short-circuit the substrate-verification we care about.
  test('grouping + Ryan-circle: 3 designer-in-5min render one avatar; Ryan renders as filled circle', async () => {
    const { app } = await launchWithIsolatedSession()
    const win = await app.firstWindow()

    const base = new Date('2026-04-22T12:00:00Z').getTime()
    const fixture = [
      {
        from: 'designer:claude-code',
        text: 'first of the group',
        ts: new Date(base).toISOString(),
        message_type: 'chat'
      },
      {
        from: 'designer:claude-code',
        text: 'second, 10s later',
        ts: new Date(base + 10_000).toISOString(),
        message_type: 'chat'
      },
      {
        from: 'designer:claude-code',
        text: 'third, 20s later',
        ts: new Date(base + 20_000).toISOString(),
        message_type: 'chat'
      },
      {
        from: 'ryan:human',
        text: 'ryan breaking the group',
        ts: new Date(base + 30_000).toISOString(),
        message_type: 'chat'
      }
    ]
    // Hydration endpoint returns reverse-chronological per server
    // convention (newest first) — see useWebSocket.ts:155 where the
    // renderer reverses it back to chronological before rendering.
    await win.route('**/api/messages**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([...fixture].reverse())
      })
    })

    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    const messageList = win.locator('[data-test="message-list"]')
    await expect(messageList).toBeVisible()

    // AC9(a): three consecutive designer messages + one ryan message
    // render four rows total, but only TWO avatars — the first
    // designer row anchors the 3-message group, the ryan row anchors
    // its own single-row group. Avatars 2 and 3 (grouped
    // continuations) are hidden.
    const avatars = win.locator('[data-test="message-avatar"]')
    await expect(avatars).toHaveCount(2, { timeout: 5_000 })
    await expect(avatars.nth(0)).toHaveAttribute('data-role', 'designer')
    await expect(avatars.nth(1)).toHaveAttribute('data-role', 'ryan')

    // AC9(b): Ryan's avatar is a filled circle (shape class
    // `message-avatar--circle` → border-radius: 50% in CSS). AI
    // avatars are rounded-squares (`message-avatar--square` →
    // border-radius: 4px). The shape class maps 1:1 to the rendered
    // geometry per MessageList.css; asserting on the class is a
    // boundary-level check that stays stable across CSS refactors
    // so long as the shape-per-role contract is intact.
    await expect(avatars.nth(1)).toHaveClass(/message-avatar--circle/)
    await expect(avatars.nth(0)).toHaveClass(/message-avatar--square/)

    await app.close()
  })
})
