import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — persistence-class — titlebar toggles', () => {
  test('density toggle survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const densityToggle = win.locator('[data-test="titlebar-density-toggle"]')
      await expect(densityToggle).toBeVisible()
      const initial = await densityToggle.getAttribute('data-density')

      consoleErrors.length = 0
      pageErrors.length = 0

      await densityToggle.click()
      const toggled = initial === 'compact' ? 'comfortable' : 'compact'
      await expect(densityToggle).toHaveAttribute('data-density', toggled)

      await win.reload()

      const densityAfterReload = win.locator('[data-test="titlebar-density-toggle"]')
      await expect(densityAfterReload).toBeVisible()
      await expect(densityAfterReload).toHaveAttribute('data-density', toggled)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('lanes toggle survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const lanesToggle = win.locator('[data-test="titlebar-lanes-toggle"]')
      await expect(lanesToggle).toBeVisible()
      const initial = await lanesToggle.getAttribute('data-lanes')

      consoleErrors.length = 0
      pageErrors.length = 0

      await lanesToggle.click()
      const toggled = initial === 'on' ? 'off' : 'on'
      await expect(lanesToggle).toHaveAttribute('data-lanes', toggled)

      await win.reload()

      const lanesAfterReload = win.locator('[data-test="titlebar-lanes-toggle"]')
      await expect(lanesAfterReload).toBeVisible()
      await expect(lanesAfterReload).toHaveAttribute('data-lanes', toggled)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('theme toggle survives window reload via documentElement dataset', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      const initial = await win.evaluate(() => document.documentElement.dataset.theme)
      expect(initial === 'dark' || initial === 'light').toBe(true)

      consoleErrors.length = 0
      pageErrors.length = 0

      const themeButton = win.locator('button.titlebar-theme-toggle')
      await themeButton.click()
      const toggled = initial === 'dark' ? 'light' : 'dark'
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe(toggled)

      await win.reload()

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      const themeAfterReload = await win.evaluate(() => document.documentElement.dataset.theme)
      expect(themeAfterReload).toBe(toggled)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
