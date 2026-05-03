import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — message @mention click', () => {
  test('clicking an @<role> pill scopes the chat recipient to that role', async ({}, testInfo) => {
    testInfo.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'first dev-authored message — establishes dev as known role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'pm:claude-code',
          text: 'pinging @dev — please look at this',
          ts: '2026-04-22T16:01:00Z',
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

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()
      await expect(list).toContainText('please look at this', { timeout: 10000 })

      const broadcastButton = win.locator('[data-test="sidebar-broadcast"]')
      await expect(broadcastButton).toHaveAttribute('aria-current', 'true')

      consoleErrors.length = 0
      pageErrors.length = 0

      const mention = win.locator('[data-test="message-mention"]').first()
      await expect(mention).toBeVisible()
      await mention.click()

      const devButton = win.locator('[data-test="sidebar-direct-dev"]')
      await expect(devButton).toHaveAttribute('aria-current', 'true')
      await expect(broadcastButton).not.toHaveAttribute('aria-current', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
