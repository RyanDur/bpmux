import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Fixture carries one message per non-chat class so we can assert
// each custom SVG glyph renders. Insights + activity + system are
// the three AC targets; chat is the no-glyph control.
const MESSAGES = [
  {
    from: 'dev:claude-code',
    text: 'plain chat — no glyph in the gutter',
    ts: '2026-04-22T16:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'insight: load-apply gap is the iteration layer',
    ts: '2026-04-22T16:00:10Z',
    to: null,
    addressee: null,
    message_type: 'insight'
  },
  {
    from: 'dev:claude-code',
    text: 'busy',
    ts: '2026-04-22T16:00:20Z',
    to: null,
    addressee: null,
    message_type: 'activity'
  },
  {
    from: 'bpmux',
    text: 'system broadcast',
    ts: '2026-04-22T16:00:30Z',
    to: null,
    addressee: null,
    message_type: 'system'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-373-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-373-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
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

test.describe('desk-check #373 illustrated class glyphs', () => {
  test('events + bpmux messages each render a labelled SVG glyph', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await win.locator('[data-test="filter-pill-events"]').click()
    await win.locator('[data-test="filter-pill-bpmux"]').click()

    const events = win.locator('[data-test="message-class-glyph"][data-class="events"]')
    await expect(events.first()).toBeVisible({ timeout: 5_000 })
    await expect(events.first()).toHaveAttribute('role', 'img')
    await expect(events.first()).toHaveAttribute('aria-label', 'events')
    expect(await events.first().evaluate((el) => el.tagName.toLowerCase())).toBe('svg')

    const bpmux = win.locator('[data-test="message-class-glyph"][data-class="bpmux"]')
    await expect(bpmux.first()).toBeVisible({ timeout: 5_000 })
    await expect(bpmux.first()).toHaveAttribute('aria-label', 'bpmux')
    expect(await bpmux.first().evaluate((el) => el.tagName.toLowerCase())).toBe('svg')

    await win.screenshot({
      path: join(EVIDENCE_DIR, '92-class-glyphs.png')
    })

    await app.close()
  })

  test('chat messages render without a class glyph', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // The chat message body should NOT contain a class-glyph span;
    // only non-chat classes decorate the gutter.
    const chatRow = win
      .locator('[data-test="message-row"]', {
        hasText: 'plain chat — no glyph in the gutter'
      })
      .first()
    await expect(chatRow).toBeVisible({ timeout: 5_000 })
    await expect(chatRow.locator('[data-test="message-class-glyph"]')).toHaveCount(0)

    await app.close()
  })
})
