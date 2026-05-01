import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — persistence-class — board collapse state', () => {
  test('user-toggled icebox expand survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const iceboxToggle = win.locator('[data-test="board-group-icebox"]')
      await expect(iceboxToggle).toHaveAttribute('aria-expanded', 'false')

      await iceboxToggle.click()
      await expect(iceboxToggle).toHaveAttribute('aria-expanded', 'true')

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const iceboxAfterReload = win.locator('[data-test="board-group-icebox"]')
      await expect(iceboxAfterReload).toHaveAttribute('aria-expanded', 'true')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('user-toggled in-progress collapse survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const inProgressToggle = win.locator('[data-test="board-group-in-progress"]')
      await expect(inProgressToggle).toHaveAttribute('aria-expanded', 'true')

      await inProgressToggle.click()
      await expect(inProgressToggle).toHaveAttribute('aria-expanded', 'false')

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const inProgressAfterReload = win.locator('[data-test="board-group-in-progress"]')
      await expect(inProgressAfterReload).toHaveAttribute('aria-expanded', 'false')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
