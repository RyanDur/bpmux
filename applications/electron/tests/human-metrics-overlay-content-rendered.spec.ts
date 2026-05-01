import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — metrics overlay content', () => {
  test('opening metrics overlay renders the Velocity + DORA header + metric tiles', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="app-root"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.keyboard.press('Control+Shift+M')

      const overlay = win.locator('[data-test="metrics-overlay"]')
      await expect(overlay).toBeVisible()
      await expect(overlay.locator('h2')).toHaveText('Velocity + DORA')

      for (const label of [
        'Delivered / day',
        'Delivered in window',
        'Median pickup → deliver',
        'Median deliver → accept',
        'Rework rate'
      ]) {
        await expect(overlay).toContainText(label, { timeout: 10000 })
      }

      const closeButton = overlay.locator('button.metrics-close')
      await closeButton.click()
      // #497: Metrics is now a native <dialog> (always mounted; open
      // state toggled via show()/close()). Closed-state assertion shifts
      // from element-unmount to el.open === false.
      await expect.poll(() => overlay.evaluate((el: HTMLDialogElement) => el.open)).toBe(false)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
