import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — sidebar empty-state', () => {
  test('empty session shows broadcast/all + groups-empty + system-empty fallbacks', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const sidebar = win.locator('[data-test="sidebar"]')
      await expect(sidebar).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(win.locator('[data-test="sidebar-all"]')).toBeVisible()
      await expect(win.locator('[data-test="sidebar-broadcast"]')).toBeVisible()
      await expect(win.locator('[data-test="sidebar-groups-empty"]')).toBeVisible()
      await expect(win.locator('[data-test="sidebar-groups-empty"]')).toContainText('none yet')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
