import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — message class glyphs', () => {
  test('insight-class seeded message renders the insight class glyph', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'an insight-class reasoning trace',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'insight'
        }
      ]
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()
      await expect(list).toContainText('reasoning trace', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      const insightGlyph = win.locator('[data-test="message-class-glyph"][data-class="insight"]')
      await expect(insightGlyph).toHaveCount(1)

      const chatGlyph = win.locator('[data-test="message-class-glyph"][data-class="chat"]')
      await expect(chatGlyph).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('system-class seeded message renders the system class glyph', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'a chat message to anchor known-roles',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'bpmux:system',
          text: 'a system event message',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'system'
        }
      ]
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.locator('[data-test="filter-pill-system"]').click()

      const systemGlyph = win.locator('[data-test="message-class-glyph"][data-class="system"]')
      await expect(systemGlyph).toHaveCount(1, { timeout: 10000 })

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
