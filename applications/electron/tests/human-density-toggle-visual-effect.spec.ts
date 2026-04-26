import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — density toggle visual', () => {
  test('clicking density-toggle flips documentElement.dataset.density', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="titlebar"]')).toBeVisible()

    const initialAttr = await win.evaluate(() => document.documentElement.dataset.density)
    expect(initialAttr === 'compact' || initialAttr === 'comfortable').toBe(true)

    consoleErrors.length = 0
    pageErrors.length = 0

    const button = win.locator('[data-test="titlebar-density-toggle"]')
    await button.click()
    const toggled = initialAttr === 'compact' ? 'comfortable' : 'compact'
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe(toggled)

    await button.click()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe(initialAttr)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
