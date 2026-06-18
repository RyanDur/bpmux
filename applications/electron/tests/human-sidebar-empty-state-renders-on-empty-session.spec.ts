import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { SidebarPage } from './pages/sidebarPage'

test.describe('human-side E2E #476 — read-class — sidebar empty-state', () => {
  test('empty session shows all-conversations + groups-empty; broadcast + system retired (#1457)', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const sidebar = new SidebarPage(win)
      await expect(sidebar.nav()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(sidebar.allChannel()).toBeVisible()
      await expect(sidebar.directChannel('broadcast')).toHaveCount(0)
      await expect(sidebar.groupsEmpty()).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
