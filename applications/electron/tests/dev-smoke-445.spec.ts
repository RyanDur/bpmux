import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow, launchIsolatedElectron } from './helpers/launch'

test.describe('#445 sidebar collapse-state persists across refresh — dev smoke', () => {
  test('collapse state survives an app restart for the same userDataDir', async () => {
    const first = await launchIsolatedWithWindow()
    const directToggle = first.win.locator('[data-test="sidebar-section-direct-toggle"]')
    await expect(directToggle).toHaveAttribute('aria-expanded', 'true')
    await directToggle.click()
    await expect(directToggle).toHaveAttribute('aria-expanded', 'false')
    const userDataDir = first.userDataDir
    const sessionBase = first.sessionBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, sessionBase })
    const win = await second.app.firstWindow()
    const directToggleAfter = win.locator('[data-test="sidebar-section-direct-toggle"]')
    await expect(directToggleAfter).toBeVisible()
    await expect(directToggleAfter).toHaveAttribute('aria-expanded', 'false')
    await second.app.close()
  })

  test('per-section state is preserved independently', async () => {
    const first = await launchIsolatedWithWindow()
    const directToggle = first.win.locator('[data-test="sidebar-section-direct-toggle"]')
    const groupsToggle = first.win.locator('[data-test="sidebar-section-groups-toggle"]')
    const systemToggle = first.win.locator('[data-test="sidebar-section-system-toggle"]')
    await directToggle.click()
    await systemToggle.click()
    await expect(directToggle).toHaveAttribute('aria-expanded', 'false')
    await expect(groupsToggle).toHaveAttribute('aria-expanded', 'true')
    await expect(systemToggle).toHaveAttribute('aria-expanded', 'false')
    const userDataDir = first.userDataDir
    const sessionBase = first.sessionBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, sessionBase })
    const win = await second.app.firstWindow()
    await expect(win.locator('[data-test="sidebar-section-direct-toggle"]')).toHaveAttribute(
      'aria-expanded',
      'false'
    )
    await expect(win.locator('[data-test="sidebar-section-groups-toggle"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    await expect(win.locator('[data-test="sidebar-section-system-toggle"]')).toHaveAttribute(
      'aria-expanded',
      'false'
    )
    await second.app.close()
  })

  test('fresh userDataDir starts with default expanded sections', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    await expect(
      win.locator('[data-test="sidebar-section-direct-toggle"]')
    ).toHaveAttribute('aria-expanded', 'true')
    await expect(
      win.locator('[data-test="sidebar-section-groups-toggle"]')
    ).toHaveAttribute('aria-expanded', 'true')
    await expect(
      win.locator('[data-test="sidebar-section-system-toggle"]')
    ).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })
})
