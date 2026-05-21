import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #388 "N new messages ↓" indicator. When chat is scrolled up from
// the bottom + new messages arrive, a floating pill shows the
// count; clicking scrolls to bottom + dismisses.
//
// Test scope (scope-honest): default-path falsifier (pill must NOT
// render at rest) is covered here. The at-count-increment path
// requires triggering an actual message-length increase under a
// scrolled-up state, which the route-based fixture can't easily
// exercise since the messages come from the WS hook at boot. The
// remaining falsifier coverage (pill appears + click dismisses)
// rides the post-#394 live-verify walk.

const SEED_MESSAGES = Array.from({ length: 10 }, (_, i) => ({
  from: 'pm:claude-code',
  text: `seed message #${i + 1}`,
  ts: new Date(2026, 3, 23, 10, i).toISOString(),
  to: null,
  addressee: null,
  message_type: 'chat'
}))

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-388-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-388-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-23T11:00:00Z', stories: [] })
    })
  })
  await win.route('**/api/messages*', async (route, request) => {
    const url = request.url()
    if (url.includes('message_type=activity')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: '[]'
      })
      return
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(SEED_MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #388 "new messages ↓" indicator', () => {
  test('pill does NOT render when user is at bottom (AC5 regression gate)', async () => {
    // Falsifier: "pill appears when user IS at bottom (default
    // case)." This guards against a future refactor that
    // accidentally renders the indicator whenever `newCount` is
    // initialized or whenever new messages arrive without scroll
    // context. At steady state + bottom-anchored, the pill must
    // stay absent.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect(win.locator('[data-test="message-list"]')).toBeVisible()

    await expect(win.locator('[data-test="new-messages-pill"]')).toHaveCount(0)

    await app.close()
  })

  test('message-list-wrap positioning context present (structural gate)', async () => {
    // The pill's absolute positioning relies on the new
    // `.message-list-wrap` div. Regression gate: verify the
    // wrapper exists and the ul lives inside it, so a future
    // refactor can't remove the positioning context + silently
    // break the pill's layout.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const wrap = win.locator('[data-test="message-list-wrap"]')
    await expect(wrap).toBeVisible()
    const inner = wrap.locator('[data-test="message-list"]')
    await expect(inner).toBeVisible()

    // position: relative on the wrap (absolute-positioning context
    // for the pill). Read computed style to lock it in.
    const position = await wrap.evaluate((el) => getComputedStyle(el).position)
    expect(position).toBe('relative')

    await app.close()
  })
})
