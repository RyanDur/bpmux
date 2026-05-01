import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — pinned list', () => {
  test('seeded pins enable the pin-count button + clicking opens the pinned list', async () => {
    const seededMessage = {
      from: 'pm:claude-code',
      text: 'PINNED_LIST_TARGET_MARKER',
      ts: '2026-04-22T16:00:00Z',
      message_type: 'chat'
    }
    const messageKey = `${seededMessage.ts}|${seededMessage.from}`
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [seededMessage],
      seedPins: {
        broadcast: [
          {
            message_key: messageKey,
            from: seededMessage.from,
            ts: seededMessage.ts,
            text: seededMessage.text,
            pinned_by: 'ryan',
            pinned_at: '2026-04-22T16:01:00Z'
          }
        ]
      }
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="message-list"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const pinCount = win.locator('[data-test="chat-pin-count"]')
      await expect(pinCount).toBeVisible()
      await expect(pinCount).toBeEnabled({ timeout: 10000 })
      await expect(pinCount).toContainText('1')

      await pinCount.click()
      const overlay = win.locator('[data-test="pinned-list-overlay"]')
      await expect(overlay).toBeVisible()
      await expect(overlay).toContainText(seededMessage.text)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
