import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — chat scope label', () => {
  test('a direct role opens its scoped chat (not a terminal); the active channel follows the selection', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'seed dev as direct-role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['dev:claude-code']
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chatRegion = win.locator('[data-test="chat-region"]')
      await expect(chatRegion).toBeVisible()
      await waitForMessagesLoaded(win, 'seed dev as direct-role')

      consoleErrors.length = 0
      pageErrors.length = 0

      const allButton = win.locator('[data-test="sidebar-all"]')
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      const devButton = win.locator('[data-test="sidebar-direct-dev"]')
      await expect(devButton).toBeVisible({ timeout: 20000 })
      await devButton.click()
      await expect(win.locator('[data-test="message-list"]')).toBeVisible({ timeout: 10000 })
      await expect(win.locator('[data-test="terminal-dev"]')).toHaveCount(0)
      await expect(devButton).toHaveAttribute('aria-current', 'true')

      await allButton.click()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('chat-warning aside renders when connection is not connected (initial connecting state)', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const warning = win.locator('[data-test="chat-warning"]')
      const connectionLabel = win.locator('[data-test="titlebar-app-state"]')
      await expect(connectionLabel).toContainText('online', { timeout: 10000 })
      await expect(warning).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
