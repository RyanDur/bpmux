import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — titlebar connection status', () => {
  test('connection status renders with one of online/connecting/offline label', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const titlebar = win.locator('[data-test="titlebar"]')
      await expect(titlebar).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const connection = win.locator('[data-test="titlebar-app"]')
      await expect(connection).toBeVisible()

      const label = win.locator('[data-test="titlebar-app-state"]')
      await expect(label).toBeVisible()
      const labelText = (await label.textContent())?.trim() ?? ''
      expect(['online', 'reconnecting', 'offline']).toContain(labelText)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('connection eventually reaches online state', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="titlebar"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const label = win.locator('[data-test="titlebar-app-state"]')
      await expect(label).toContainText('online', { timeout: 10000 })

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
