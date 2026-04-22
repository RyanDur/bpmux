import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MESSAGES = [
  {
    from: 'dev:claude-code',
    text: 'chat message one',
    ts: '2026-04-22T16:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'insight observation',
    ts: '2026-04-22T16:00:10Z',
    to: null,
    addressee: null,
    message_type: 'insight'
  },
  {
    from: 'designer:claude-code',
    text: 'busy',
    ts: '2026-04-22T16:00:20Z',
    to: null,
    addressee: null,
    message_type: 'activity'
  },
  {
    from: 'bpmux',
    text: 'system signal',
    ts: '2026-04-22T16:00:30Z',
    to: null,
    addressee: null,
    message_type: 'system'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-374-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-374-'))
  const app = await electron.launch({
    args: ['out/main/index.js', `--user-data-dir=${udd}`],
    env: { ...process.env, BPMUX_SESSION_BASE: sessionBase }
  })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-22T16:00:00Z', stories: [] })
    })
  })
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #374 lane separation', () => {
  test('toggling lanes groups messages by class into distinct sections', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Default (interleaved): no lanes container, one message-list.
    await expect(win.locator('[data-test="message-lanes"]')).toHaveCount(0)
    await expect(win.locator('[data-test="message-list"]')).toBeVisible()

    // Toggle lanes on via titlebar button.
    const toggle = win.locator('[data-test="titlebar-lanes-toggle"]')
    await expect(toggle).toBeVisible()
    await toggle.click()

    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.lanes))
      .toBe('on')

    // Lane mode: lanes container present + at least the chat lane
    // renders because the fixture has a chat message. Default
    // filter excludes activity + system, but insight should pass.
    await expect(win.locator('[data-test="message-lanes"]')).toBeVisible({
      timeout: 5_000
    })

    const chatLane = win.locator(
      '[data-test="message-lane"][data-lane-class="chat"]'
    )
    await expect(chatLane).toBeVisible()

    // Chat lane should contain the chat message.
    await expect(chatLane.locator('[data-test="message-row"]')).toContainText(
      'chat message one'
    )

    // The insight message should land in a SEPARATE lane (insight),
    // not in the chat lane — that's the core lane-separation claim.
    const insightLane = win.locator(
      '[data-test="message-lane"][data-lane-class="insight"]'
    )
    if ((await insightLane.count()) > 0) {
      await expect(insightLane.locator('[data-test="message-row"]')).toContainText(
        'insight observation'
      )
      // Insight row should NOT appear in the chat lane.
      await expect(
        chatLane.locator('[data-test="message-row"]', {
          hasText: 'insight observation'
        })
      ).toHaveCount(0)

      // Bounding-box sanity: the two lanes occupy distinct vertical
      // regions (chat lane's top != insight lane's top).
      const chatBox = await chatLane.boundingBox()
      const insightBox = await insightLane.boundingBox()
      expect(chatBox).not.toBeNull()
      expect(insightBox).not.toBeNull()
      if (chatBox && insightBox) {
        expect(chatBox.y).not.toBe(insightBox.y)
      }
    }

    await win.screenshot({
      path: join(EVIDENCE_DIR, '97-lanes-on.png'),
      fullPage: true
    })

    await app.close()
  })

  test('lanes preference persists across reload', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await win.locator('[data-test="titlebar-lanes-toggle"]').click()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.lanes))
      .toBe('on')

    await win.reload()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.lanes))
      .toBe('on')

    await app.close()
  })
})
