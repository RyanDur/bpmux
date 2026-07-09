import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

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
  message_type: 'chat'
}))

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-388-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-388-'))
  const { app } = await launchIsolatedElectron({
    dataBase,
    userDataDir: udd,
    seedMessages: SEED_MESSAGES
  })
  const win = await app.firstWindow()
  await waitForMessagesLoaded(win, 'seed message #10')
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
    const chat = new ChatPage(win)
    await expect(chat.region()).toBeVisible()
    await expect(chat.messageScroller()).toBeVisible()

    await expect(chat.newMessagesPill()).toHaveCount(0)

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
    const chat = new ChatPage(win)
    const wrap = chat.messageListWrap()
    await expect(wrap).toBeVisible()
    await expect(wrap.getByRole('list')).toBeVisible()

    const position = await wrap.evaluate((el) => getComputedStyle(el).position)
    expect(position).toBe('relative')

    await app.close()
  })
})
