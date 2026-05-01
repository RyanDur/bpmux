import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — message reaction picker', () => {
  test('reaction-add → pick emoji → pill renders on the message (full cycle)', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'reactable test message',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
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

      await expect(win.locator('[data-test="message-list"]')).toBeVisible()
      await expect(win.locator('[data-test="message-list"]')).toContainText(
        'reactable test message',
        { timeout: 10000 }
      )

      consoleErrors.length = 0
      pageErrors.length = 0

      const addButton = win.locator('[data-test="reaction-add-first"]').first()
      await expect(addButton).toBeVisible()
      await expect(addButton).toHaveAttribute('aria-expanded', 'false')

      await addButton.click()
      await expect(addButton).toHaveAttribute('aria-expanded', 'true')

      const picker = win.locator('[data-test="reaction-picker"]')
      await expect(picker).toBeVisible()

      const thumbsUp = win.locator('[data-test="reaction-picker-👍"]')
      await expect(thumbsUp).toBeVisible()
      await thumbsUp.click()

      await expect(picker).toHaveCount(0)

      const pill = win.locator('[data-test="reaction-pill-👍"]')
      await expect(pill).toBeVisible({ timeout: 10000 })

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
