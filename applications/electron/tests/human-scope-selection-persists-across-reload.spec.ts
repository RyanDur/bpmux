import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { SidebarPage } from './pages/sidebarPage'

test.describe('human-side E2E #476 — persistence-class — scope selection', () => {
  test('all-view selection survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedRoster: ['dev:claude-code', 'pm:claude-code']
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const sidebar = new SidebarPage(win)
      const allButton = sidebar.allChannel()
      await expect(allButton).toBeVisible()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      consoleErrors.length = 0
      pageErrors.length = 0

      const devButton = sidebar.directChannel('dev')
      await devButton.click()
      await expect(devButton).toHaveAttribute('aria-current', 'true')
      await allButton.click()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      await win.reload()

      const allAfterReload = sidebar.allChannel()
      await expect(allAfterReload).toBeVisible()
      await expect(allAfterReload).toHaveAttribute('aria-current', 'true')
      const devAfterReload = sidebar.directChannel('dev')
      await expect(devAfterReload).not.toHaveAttribute('aria-current', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('all-conversations default scope survives reload when never toggled', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const sidebar = new SidebarPage(win)
      const allButton = sidebar.allChannel()
      await expect(allButton).toBeVisible()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.reload()

      const allAfterReload = sidebar.allChannel()
      await expect(allAfterReload).toBeVisible()
      await expect(allAfterReload).toHaveAttribute('aria-current', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
