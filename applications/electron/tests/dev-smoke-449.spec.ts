import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow, launchIsolatedElectron } from './helpers/launch'
import { seedCurrentIterationViaTestApi } from './helpers/iteration-fixture'

test.describe('#449 board collapse-state persists across refresh — dev smoke', () => {
  test('ready column collapsed state survives app restart', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    await seedCurrentIterationViaTestApi(first.backendPort, first.authToken, 'DevSmoke449-1a')
    await first.win.reload()
    const readyToggle = first.win.locator('[data-test="board-group-ready"]')
    await expect(readyToggle).toHaveAttribute('aria-expanded', 'true')
    await readyToggle.click()
    await expect(readyToggle).toHaveAttribute('aria-expanded', 'false')
    const userDataDir = first.userDataDir
    const sessionBase = first.sessionBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, sessionBase })
    const win = await second.app.firstWindow()
    await seedCurrentIterationViaTestApi(second.backendPort, second.authToken, 'DevSmoke449-1b')
    await win.reload()
    const readyToggleAfter = win.locator('[data-test="board-group-ready"]')
    await expect(readyToggleAfter).toBeVisible()
    await expect(readyToggleAfter).toHaveAttribute('aria-expanded', 'false')
    await second.app.close()
  })

  test('per-column state is preserved independently across restart', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    await seedCurrentIterationViaTestApi(first.backendPort, first.authToken, 'DevSmoke449-2a')
    await first.win.reload()
    const ready = first.win.locator('[data-test="board-group-ready"]')
    const inProgress = first.win.locator('[data-test="board-group-in-progress"]')
    const delivered = first.win.locator('[data-test="board-group-delivered"]')
    await ready.click()
    await delivered.click()
    await expect(ready).toHaveAttribute('aria-expanded', 'false')
    await expect(inProgress).toHaveAttribute('aria-expanded', 'true')
    await expect(delivered).toHaveAttribute('aria-expanded', 'false')
    const userDataDir = first.userDataDir
    const sessionBase = first.sessionBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, sessionBase })
    const win = await second.app.firstWindow()
    await seedCurrentIterationViaTestApi(second.backendPort, second.authToken, 'DevSmoke449-2b')
    await win.reload()
    await expect(win.locator('[data-test="board-group-ready"]')).toHaveAttribute(
      'aria-expanded',
      'false'
    )
    await expect(win.locator('[data-test="board-group-in-progress"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    await expect(win.locator('[data-test="board-group-delivered"]')).toHaveAttribute(
      'aria-expanded',
      'false'
    )
    await second.app.close()
  })

  test('fresh userDataDir starts with GROUP_ORDER defaults (accepted collapsed; icebox open)', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke449-3')
    await win.reload()
    await expect(win.locator('[data-test="board-group-ready"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    await expect(win.locator('[data-test="board-group-in-progress"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    await expect(win.locator('[data-test="board-group-delivered"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
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

  test('expanding a default-collapsed column persists across restart', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    await first.win.locator('[data-test="board-tab-done"]').click()
    const accepted = first.win.locator('[data-test="board-group-accepted"]')
    await expect(accepted).toHaveAttribute('aria-expanded', 'false')
    await accepted.click()
    await expect(accepted).toHaveAttribute('aria-expanded', 'true')
    const userDataDir = first.userDataDir
    const sessionBase = first.sessionBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, sessionBase })
    const win = await second.app.firstWindow()
    await win.locator('[data-test="board-tab-done"]').click()
    await expect(win.locator('[data-test="board-group-accepted"]')).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    await second.app.close()
  })
})
