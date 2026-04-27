import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('#505 slice 5c — Board panel DESK-CHECK column', () => {
  test('column renders between IN PROGRESS and DELIVERED', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const expectedColumnOrder = [
      'board-group-ready',
      'board-group-in-progress',
      'board-group-desk-check',
      'board-group-delivered',
      'board-group-accepted',
      'board-group-icebox'
    ]

    const order = await win.evaluate(() => {
      const summaries = document.querySelectorAll<HTMLElement>(
        '[data-test^="board-group-"]:not([data-test$="-count"])'
      )
      return Array.from(summaries).map((el) => el.getAttribute('data-test') ?? '')
    })

    expect(order).toEqual(expectedColumnOrder)

    await app.close()
  })

  test('DESK-CHECK column renders empty-state with (0) count when no stories present', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const deskCheckSummary = win.locator('[data-test="board-group-desk-check"]')
    await expect(deskCheckSummary).toBeVisible()

    const count = win.locator('[data-test="board-group-desk-check-count"]')
    await expect(count).toHaveText('(0)')
    await expect(count).toHaveAttribute('data-empty', 'true')

    await app.close()
  })

  test('DESK-CHECK column header has matching styling to other columns', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const deskCheck = win.locator('[data-test="board-group-desk-check"]')
    const inProgress = win.locator('[data-test="board-group-in-progress"]')

    const deskCheckPosition = await deskCheck.evaluate((el) => getComputedStyle(el).position)
    const inProgressPosition = await inProgress.evaluate((el) => getComputedStyle(el).position)
    expect(deskCheckPosition).toBe(inProgressPosition)

    const deskCheckBg = await deskCheck.evaluate((el) => getComputedStyle(el).backgroundColor)
    const inProgressBg = await inProgress.evaluate((el) => getComputedStyle(el).backgroundColor)
    expect(deskCheckBg).toBe(inProgressBg)

    await app.close()
  })

  test('no console errors when DESK-CHECK column renders empty', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="board-region"]')).toBeVisible()
    await expect(win.locator('[data-test="board-group-desk-check"]')).toBeVisible()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
