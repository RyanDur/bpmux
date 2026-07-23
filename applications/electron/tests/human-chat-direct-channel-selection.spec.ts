import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'
import { SidebarPage } from './pages/sidebarPage'

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

      const chat = new ChatPage(win)
      const sidebar = new SidebarPage(win)
      await expect(chat.input()).toBeVisible()
      await waitForMessagesLoaded(win, 'seed dev as direct-role')

      consoleErrors.length = 0
      pageErrors.length = 0

      const allButton = sidebar.allChannel()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      const devButton = sidebar.directChannel('dev')
      await expect(devButton).toBeVisible({ timeout: 20000 })
      await devButton.click()
      await expect(chat.messageLog()).toBeVisible({ timeout: 10000 })
      await expect(chat.terminalOutput()).toHaveCount(0)
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

      const chat = new ChatPage(win)
      await expect(chat.input()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(chat.connectionState()).toHaveText('online', { timeout: 10000 })
      await expect(chat.connectionWarning()).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
