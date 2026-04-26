import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — app-level keyboard shortcuts', () => {
  test('Ctrl+P toggles board panel visibility', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const board = win.locator('[data-test="board-region"]')
    await expect(board).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const titlebar = win.locator('[data-test="titlebar"]')
    await titlebar.click()

    await win.keyboard.press('Control+p')
    await expect(board).toBeHidden()

    await win.keyboard.press('Control+p')
    await expect(board).toBeVisible()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('Ctrl+Shift+M toggles metrics overlay visibility', async () => {
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

    const overlay = win.locator('[data-test="metrics-overlay"]')
    await expect(overlay).toHaveCount(0)

    await win.keyboard.press('Control+Shift+M')
    await expect(overlay).toBeVisible()

    await win.keyboard.press('Control+Shift+M')
    await expect(overlay).toHaveCount(0)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
