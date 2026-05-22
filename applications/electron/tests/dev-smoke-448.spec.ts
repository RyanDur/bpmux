import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('#448 board column collapse affordance — dev smoke', () => {
  test('each board column renders a toggle button (across NOW/DONE/PLAN tabs)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    for (const state of ['ready', 'in-progress', 'delivered']) {
      await expect(win.locator(`[data-test="board-group-${state}"]`)).toBeVisible()
    }
    await win.locator('[data-test="board-tab-done"]').click()
    await expect(win.locator('[data-test="board-group-accepted"]')).toBeVisible()
    await win.locator('[data-test="board-tab-plan"]').click()
    await expect(win.locator('[data-test="board-group-icebox"]')).toBeVisible()
    await app.close()
  })

  test('ready column click toggles aria-expanded', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const toggle = win.locator('[data-test="board-group-ready"]')
    await expect(toggle).toBeVisible()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await toggle.click()
    await expect(toggle).toHaveAttribute('aria-expanded', 'false')
    await toggle.click()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('default collapsed columns (accepted) start closed', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    await win.locator('[data-test="board-tab-done"]').click()
    await expect(win.locator('[data-test="board-group-accepted"]')).toHaveAttribute(
      'aria-expanded',
      'false'
    )
    await win.locator('[data-test="board-tab-plan"]').click()
    await expect(win.locator('[data-test="board-group-icebox"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    await app.close()
  })

  test('board column toggle is keyboard-activatable', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const toggle = win.locator('[data-test="board-group-ready"]')
    await toggle.focus()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await win.keyboard.press('Enter')
    await expect(toggle).toHaveAttribute('aria-expanded', 'false')
    await win.keyboard.press('Space')
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('collapsed column hides its story content', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const toggle = win.locator('[data-test="board-group-ready"]')
    const content = win.locator('#board-group-ready-content')
    await expect(content).toBeVisible()
    await toggle.click()
    await expect(content).not.toBeVisible()
    await app.close()
  })

  test('chevron glyph swaps between expanded and collapsed', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const chevron = win.locator('[data-test="board-group-ready"]').locator('.board-group-chevron')
    const before = (): Promise<string> =>
      chevron.evaluate((el) => getComputedStyle(el, '::before').content)
    expect(await before()).toBe('"▾"')
    await win.locator('[data-test="board-group-ready"]').click()
    expect(await before()).toBe('"▸"')
    await app.close()
  })

  test('sidebar sections no longer expose collapse toggles — retired per #448 scope', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    await expect(win.locator('[data-test="sidebar-section-direct-toggle"]')).toHaveCount(0)
    await expect(win.locator('[data-test="sidebar-section-groups-toggle"]')).toHaveCount(0)
    await expect(win.locator('[data-test="sidebar-section-system-toggle"]')).toHaveCount(0)
    await app.close()
  })
})
