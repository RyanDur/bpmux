import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — persistence-class — message filter', () => {
  test('toggling activity-class on persists across reload', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const filterPills = win.locator('[data-test="filter-pills"]')
    await expect(filterPills).toBeVisible()

    const activityPill = win.locator('[data-test="filter-pill-activity"]')
    await expect(activityPill).toHaveAttribute('aria-pressed', 'false')

    consoleErrors.length = 0
    pageErrors.length = 0

    await activityPill.click()
    await expect(activityPill).toHaveAttribute('aria-pressed', 'true')

    await win.reload()

    const activityAfterReload = win.locator('[data-test="filter-pill-activity"]')
    await expect(activityAfterReload).toBeVisible()
    await expect(activityAfterReload).toHaveAttribute('aria-pressed', 'true')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('toggling chat-class off persists across reload', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const chatPill = win.locator('[data-test="filter-pill-chat"]')
    await expect(chatPill).toHaveAttribute('aria-pressed', 'true')

    consoleErrors.length = 0
    pageErrors.length = 0

    await chatPill.click()
    await expect(chatPill).toHaveAttribute('aria-pressed', 'false')

    await win.reload()

    const chatAfterReload = win.locator('[data-test="filter-pill-chat"]')
    await expect(chatAfterReload).toBeVisible()
    await expect(chatAfterReload).toHaveAttribute('aria-pressed', 'false')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
