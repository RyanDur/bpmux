import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — metrics overlay content', () => {
  test('opening metrics overlay renders the Velocity + DORA header + metric tiles', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

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
    await expect(overlay).toHaveCount(0)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
