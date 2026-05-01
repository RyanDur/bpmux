import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — lanes toggle visual', () => {
  test('clicking lanes-toggle flips documentElement.dataset.lanes + button data-lanes', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="titlebar"]')).toBeVisible()

      const initialAttr = await win.evaluate(() => document.documentElement.dataset.lanes)
      expect(initialAttr === 'on' || initialAttr === 'off').toBe(true)

      consoleErrors.length = 0
      pageErrors.length = 0

      const button = win.locator('[data-test="titlebar-lanes-toggle"]')
      await expect(button).toHaveAttribute('data-lanes', initialAttr ?? '')

      await button.click()
      const toggled = initialAttr === 'on' ? 'off' : 'on'
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.lanes))
        .toBe(toggled)
      await expect(button).toHaveAttribute('data-lanes', toggled)

      await button.click()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.lanes))
        .toBe(initialAttr)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('lanes=on renders message-lanes container in MessageList with seeded multi-class messages', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'a chat-class message for lanes test',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'dev:claude-code',
          text: 'an insight-class reasoning trace for lanes test',
          ts: '2026-04-22T16:01:00Z',
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

      await expect(win.locator('[data-test="message-list"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const lanesToggle = win.locator('[data-test="titlebar-lanes-toggle"]')
      if ((await lanesToggle.getAttribute('data-lanes')) !== 'on') {
        await lanesToggle.click()
      }
      await expect(lanesToggle).toHaveAttribute('data-lanes', 'on')

      const lanesContainer = win.locator('[data-test="message-lanes"]')
      await expect(lanesContainer).toBeVisible({ timeout: 10000 })

      const lanes = win.locator('[data-test="message-lane"]')
      expect(await lanes.count()).toBeGreaterThanOrEqual(2)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
