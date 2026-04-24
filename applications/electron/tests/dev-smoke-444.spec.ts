import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('#444 sidebar collapse affordance — dev smoke', () => {
  test('direct section toggles collapsed via click', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const toggle = win.locator('[data-test="sidebar-section-direct-toggle"]')
    await expect(toggle).toBeVisible()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await toggle.click()
    await expect(toggle).toHaveAttribute('aria-expanded', 'false')
    await toggle.click()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('all three sections render toggle affordances', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    await expect(win.locator('[data-test="sidebar-section-direct-toggle"]')).toBeVisible()
    await expect(win.locator('[data-test="sidebar-section-groups-toggle"]')).toBeVisible()
    await expect(win.locator('[data-test="sidebar-section-system-toggle"]')).toBeVisible()
    await app.close()
  })

  test('section toggle is keyboard-activatable', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const toggle = win.locator('[data-test="sidebar-section-direct-toggle"]')
    await toggle.focus()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await win.keyboard.press('Enter')
    await expect(toggle).toHaveAttribute('aria-expanded', 'false')
    await win.keyboard.press('Space')
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('collapsed section hides its entries', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const toggle = win.locator('[data-test="sidebar-section-groups-toggle"]')
    const content = win.locator('#sidebar-section-groups-content')
    await expect(content).toBeVisible()
    await toggle.click()
    await expect(content).toHaveCount(0)
    await app.close()
  })

  test('chevron glyph present in header as visual affordance', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const chevron = win
      .locator('[data-test="sidebar-section-direct-toggle"]')
      .locator('.sidebar-heading-chevron')
    await expect(chevron).toBeVisible()
    await expect(chevron).toHaveText('▾')
    await win.locator('[data-test="sidebar-section-direct-toggle"]').click()
    await expect(chevron).toHaveText('▸')
    await app.close()
  })
})
