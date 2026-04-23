import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #332 rework: clicking a sidebar role (designer) must:
// 1. show a "DM → designer" banner
// 2. filter the MessageList to the DM-with-designer channel
//    (outgoing me→designer + incoming designer→me, nothing else)

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Fixture: three messages. Broadcast (should disappear in DM view),
// DM me→designer, DM designer→me. Extra noise from pm to ensure
// the filter is strict.
const MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'broadcast hello',
    ts: '2026-04-22T16:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ryan:electron',
    text: 'note to designer',
    ts: '2026-04-22T16:01:00Z',
    to: ['designer'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'reply from designer',
    ts: '2026-04-22T16:02:00Z',
    to: ['ryan'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'pm note to ryan',
    ts: '2026-04-22T16:03:00Z',
    to: ['ryan'],
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-332-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-332-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-22T16:00:00Z', stories: [] })
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
      body: JSON.stringify(MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #332 rework: sidebar DM scope', () => {
  test('clicking designer filters chat + shows DM banner', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Before click: broadcast view — all 4 messages render.
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(4, {
      timeout: 5_000
    })
    // No DM banner yet.
    await expect(win.locator('.chat-scope')).toHaveCount(0)

    // Click the designer sidebar row.
    const designerRow = win
      .locator('[data-test="sidebar-direct"]', { hasText: 'designer' })
      .or(win.locator('button', { hasText: 'designer' }).first())
    await designerRow.first().click()

    // Banner should appear.
    await expect(win.locator('.chat-scope')).toBeVisible()
    await expect(win.locator('.chat-scope')).toContainText('designer')

    // Message list: only the 2 DM-with-designer messages remain.
    // Broadcast + pm→ryan DM should be filtered out.
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(2, {
      timeout: 3_000
    })
    await expect(
      win.locator('[data-test="message-row"]', { hasText: 'note to designer' })
    ).toBeVisible()
    await expect(
      win.locator('[data-test="message-row"]', { hasText: 'reply from designer' })
    ).toBeVisible()
    // Negative: broadcast + pm DM NOT in the scoped view.
    await expect(
      win.locator('[data-test="message-row"]', { hasText: 'broadcast hello' })
    ).toHaveCount(0)
    await expect(
      win.locator('[data-test="message-row"]', { hasText: 'pm note to ryan' })
    ).toHaveCount(0)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '101-dm-scope-designer.png')
    })
    await app.close()
  })
})
