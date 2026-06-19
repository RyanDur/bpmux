import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'
import { seedCurrentIterationViaTestApi } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { SidebarPage } from './pages/sidebarPage'

test.describe('#448 board column collapse affordance — dev smoke', () => {
  test('each board column renders a toggle button (across NOW/DONE/PLAN tabs)', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke448-1')
    await win.reload()
    const board = new BoardPage(win)
    for (const label of ['Ready', 'In Progress', 'Acceptance']) {
      await expect(board.columnToggle(label)).toBeVisible()
    }
    await board.openTab('Done')
    await expect(board.columnToggle('Accepted')).toBeVisible()
    await board.openTab('Plan')
    await expect(board.columnToggle('Icebox')).toBeVisible()
    await app.close()
  })

  test('ready column click toggles aria-expanded', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke448-2')
    await win.reload()
    const toggle = new BoardPage(win).columnToggle('Ready')
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
    const board = new BoardPage(win)
    await board.openTab('Done')
    await expect(board.columnToggle('Accepted')).toHaveAttribute('aria-expanded', 'false')
    await board.openTab('Plan')
    await expect(board.columnToggle('Icebox')).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('board column toggle is keyboard-activatable', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke448-4')
    await win.reload()
    const toggle = new BoardPage(win).columnToggle('Ready')
    await toggle.focus()
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await win.keyboard.press('Enter')
    await expect(toggle).toHaveAttribute('aria-expanded', 'false')
    await win.keyboard.press('Space')
    await expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await app.close()
  })

  test('collapsed column hides its story content', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke448-5')
    await win.reload()
    const toggle = new BoardPage(win).columnToggle('Ready')
    const content = win.locator('#board-group-ready-content')
    await expect(content).toBeVisible()
    await toggle.click()
    await expect(content).not.toBeVisible()
    await app.close()
  })

  test('chevron glyph swaps between expanded and collapsed', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow()
    await seedCurrentIterationViaTestApi(backendPort, authToken, 'DevSmoke448-6')
    await win.reload()
    const toggle = new BoardPage(win).columnToggle('Ready')
    const chevron = toggle.locator('.board-group-chevron')
    const before = (): Promise<string> =>
      chevron.evaluate((el) => getComputedStyle(el, '::before').content)
    expect(await before()).toBe('"▾"')
    await toggle.click()
    expect(await before()).toBe('"▸"')
    await app.close()
  })

  test('sidebar sections no longer expose collapse toggles — retired per #448 scope', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    await expect(new SidebarPage(win).nav().locator('summary')).toHaveCount(0)
    await app.close()
  })
})
