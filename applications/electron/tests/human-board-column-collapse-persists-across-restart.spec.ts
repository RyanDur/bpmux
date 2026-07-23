import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow, launchIsolatedElectron } from './helpers/launch'
import { seedCurrentIterationViaTestApi } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

test.describe('#449 board collapse-state persists across refresh — dev smoke', () => {
  test('ready column collapsed state survives app restart', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    await seedCurrentIterationViaTestApi(first.backendPort, first.authToken, 'DevSmoke449-1a')
    await first.win.reload()
    const readyToggle = new BoardPage(first.win).columnToggle('Ready')
    await expect(readyToggle).toHaveAttribute('aria-expanded', 'true')
    await readyToggle.click()
    await expect(readyToggle).toHaveAttribute('aria-expanded', 'false')
    const userDataDir = first.userDataDir
    const dataBase = first.dataBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, dataBase })
    const win = await second.app.firstWindow()
    await seedCurrentIterationViaTestApi(second.backendPort, second.authToken, 'DevSmoke449-1b')
    await win.reload()
    const readyToggleAfter = new BoardPage(win).columnToggle('Ready')
    await expect(readyToggleAfter).toBeVisible()
    await expect(readyToggleAfter).toHaveAttribute('aria-expanded', 'false')
    await second.app.close()
  })

  test('per-column state is preserved independently across restart', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    await seedCurrentIterationViaTestApi(first.backendPort, first.authToken, 'DevSmoke449-2a')
    await first.win.reload()
    const firstBoard = new BoardPage(first.win)
    const ready = firstBoard.columnToggle('Ready')
    const inProgress = firstBoard.columnToggle('In Progress')
    const delivered = firstBoard.columnToggle('Acceptance')
    await ready.click()
    await delivered.click()
    await expect(ready).toHaveAttribute('aria-expanded', 'false')
    await expect(inProgress).toHaveAttribute('aria-expanded', 'true')
    await expect(delivered).toHaveAttribute('aria-expanded', 'false')
    const userDataDir = first.userDataDir
    const dataBase = first.dataBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, dataBase })
    const win = await second.app.firstWindow()
    await seedCurrentIterationViaTestApi(second.backendPort, second.authToken, 'DevSmoke449-2b')
    await win.reload()
    const secondBoard = new BoardPage(win)
    await expect(secondBoard.columnToggle('Ready')).toHaveAttribute('aria-expanded', 'false')
    await expect(secondBoard.columnToggle('In Progress')).toHaveAttribute('aria-expanded', 'true')
    await expect(secondBoard.columnToggle('Acceptance')).toHaveAttribute('aria-expanded', 'false')
    await second.app.close()
  })

  test('fresh userDataDir starts with GROUP_ORDER defaults (accepted collapsed; icebox open)', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke449-3')
    await win.reload()
    const board = new BoardPage(win)
    await expect(board.columnToggle('Ready')).toHaveAttribute('aria-expanded', 'true')
    await expect(board.columnToggle('In Progress')).toHaveAttribute('aria-expanded', 'true')
    await expect(board.columnToggle('Acceptance')).toHaveAttribute('aria-expanded', 'true')
    await board.openTab('Done')
    await expect(board.columnToggle('Accepted')).toHaveAttribute('aria-expanded', 'false')
    await board.openTab('Plan')
    await expect(board.columnToggle('Icebox')).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('expanding a default-collapsed column persists across restart', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    const firstBoard = new BoardPage(first.win)
    await firstBoard.openTab('Done')
    const accepted = firstBoard.columnToggle('Accepted')
    await expect(accepted).toHaveAttribute('aria-expanded', 'false')
    await accepted.click()
    await expect(accepted).toHaveAttribute('aria-expanded', 'true')
    const userDataDir = first.userDataDir
    const dataBase = first.dataBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, dataBase })
    const win = await second.app.firstWindow()
    const secondBoard = new BoardPage(win)
    await secondBoard.openTab('Done')
    await expect(secondBoard.columnToggle('Accepted')).toHaveAttribute('aria-expanded', 'true')
    await second.app.close()
  })
})
