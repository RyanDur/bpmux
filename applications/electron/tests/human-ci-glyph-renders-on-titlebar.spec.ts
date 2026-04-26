import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — CI glyph', () => {
  test('CI glyph renders in titlebar with a conclusion attribute', async () => {
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

    const glyph = win.locator('[data-test="ci-glyph"]')
    await expect(glyph).toBeVisible()
    const conclusion = await glyph.getAttribute('data-conclusion')
    expect(conclusion).not.toBeNull()
    expect(conclusion?.length ?? 0).toBeGreaterThan(0)

    await app.close()
  })
})
