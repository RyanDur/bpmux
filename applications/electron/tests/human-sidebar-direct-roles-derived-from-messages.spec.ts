import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'
import { SidebarPage } from './pages/sidebarPage'

test.describe('human-side E2E #476 — direct-action — sidebar direct-roles', () => {
  test('seeded messages from multiple roles populate sidebar direct-role buttons', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'pm broadcasts to seed direct-role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'dev:claude-code',
          text: 'dev broadcasts to seed direct-role',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'chat'
        },
        {
          from: 'designer:claude-code',
          text: 'designer broadcasts to seed direct-role',
          ts: '2026-04-22T16:02:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['pm:claude-code', 'dev:claude-code', 'designer:claude-code']
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const sidebar = new SidebarPage(win)
      await expect(sidebar.nav()).toBeVisible()
      await waitForMessagesLoaded(win, 'pm broadcasts to seed direct-role')

      consoleErrors.length = 0
      pageErrors.length = 0

      for (const role of ['pm', 'dev', 'designer']) {
        const button = sidebar.directChannel(role)
        await expect(button).toBeVisible({ timeout: 20000 })
        await expect(button).toContainText(role)
      }

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('clicking a direct-role button scopes the sidebar to that role', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'pm broadcast',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'dev:claude-code',
          text: 'dev broadcast',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['pm:claude-code', 'dev:claude-code']
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const sidebar = new SidebarPage(win)
      await expect(sidebar.nav()).toBeVisible()
      await waitForMessagesLoaded(win, 'pm broadcast')

      const allButton = sidebar.allChannel()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      consoleErrors.length = 0
      pageErrors.length = 0

      const devButton = sidebar.directChannel('dev')
      await expect(devButton).toBeVisible({ timeout: 20000 })
      await devButton.click()
      await expect(devButton).toHaveAttribute('aria-current', 'true')
      await expect(allButton).not.toHaveAttribute('aria-current', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
