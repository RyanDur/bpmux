import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — persistence-class — scope selection', () => {
  test('all-view selection survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const broadcastButton = win.locator('[data-test="sidebar-broadcast"]')
      await expect(broadcastButton).toBeVisible()
      await expect(broadcastButton).toHaveAttribute('aria-current', 'true')

      consoleErrors.length = 0
      pageErrors.length = 0

      const allButton = win.locator('[data-test="sidebar-all"]')
      await allButton.click()
      await expect(allButton).toHaveAttribute('aria-current', 'true')

      await win.reload()

      const allAfterReload = win.locator('[data-test="sidebar-all"]')
      await expect(allAfterReload).toBeVisible()
      await expect(allAfterReload).toHaveAttribute('aria-current', 'true')
      const broadcastAfterReload = win.locator('[data-test="sidebar-broadcast"]')
      await expect(broadcastAfterReload).not.toHaveAttribute('aria-current', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('broadcast-default scope survives reload when never toggled', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const broadcastButton = win.locator('[data-test="sidebar-broadcast"]')
      await expect(broadcastButton).toBeVisible()
      await expect(broadcastButton).toHaveAttribute('aria-current', 'true')

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.reload()

      const broadcastAfterReload = win.locator('[data-test="sidebar-broadcast"]')
      await expect(broadcastAfterReload).toBeVisible()
      await expect(broadcastAfterReload).toHaveAttribute('aria-current', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
