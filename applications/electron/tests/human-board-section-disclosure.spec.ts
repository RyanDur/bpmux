import {
  launchWithMockGhAndSeededIteration,
  seedCurrentIterationViaRpc,
  seedCurrentIterationViaTestApi
} from './helpers/iteration-fixture'
import {
  launchIsolatedElectron,
  launchIsolatedWithWindow,
  launchWithMockGh,
  waitForBoardLoaded
} from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { SidebarPage } from './pages/sidebarPage'
import { expect, test } from '@playwright/test'

test.describe('#468 board panel B2 — section-count badges', () => {
  test('badge visible in collapsed section header', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')

      await expect(board.columnToggle('Accepted')).toHaveAttribute('aria-expanded', 'false')

      await expect(board.columnCount('Accepted')).toBeVisible()
      await expect(board.columnCount('Accepted')).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('badge visible in expanded section header', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeExpanded')
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.columnToggle('Ready')).toHaveAttribute('aria-expanded', 'true')

      await expect(board.columnCount('Ready')).toBeVisible()
      await expect(board.columnCount('Ready')).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('badge format is (N) integer-in-parens', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeFormat')
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')
      await board.columnToggle('Accepted').click()
      await board.openTab('Plan')
      await board.openTab('Now')

      for (const label of ['Ready', 'In Progress', 'Acceptance']) {
        await expect(board.columnCount(label)).toHaveText(/^\(\d+\)$/)
      }
      await board.openTab('Done')
      await expect(board.columnCount('Accepted')).toHaveText(/^\(\d+\)$/)
      await board.openTab('Plan')
      await expect(board.columnCount('Icebox')).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('count matches the visible story rows in expanded section', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeCountMatch')
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.columnCount('Ready')).toHaveText(/^\(\d+\)$/)
      const countText = await board.columnCount('Ready').innerText()
      const n = Number.parseInt(countText.replace(/[()]/g, ''), 10)
      await expect(board.group('Ready').locator('[aria-label^="Open story #"]')).toHaveCount(n)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('empty (count=0) badge carries data-empty and renders in --fg-dim', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeEmpty')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const ready = board.columnCount('Ready')
      await expect(ready).toHaveText('(0)')
      await expect(ready).toHaveAttribute('data-empty', 'true')

      const dimRgb = await win.evaluate(() => {
        const probe = document.createElement('span')
        probe.style.color = 'var(--fg-dim)'
        document.body.appendChild(probe)
        const c = getComputedStyle(probe).color
        probe.remove()
        return c
      })
      const renderedColor = await ready.evaluate((el) => getComputedStyle(el).color)
      expect(renderedColor).toBe(dimRgb)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('clicking the count badge toggles the section (Coyle entire-area-clickable)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeClickToggle')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const summary = board.columnToggle('Ready')
      const count = board.columnCount('Ready')

      await expect(summary).toHaveAttribute('aria-expanded', 'true')
      await count.click()
      await expect(summary).toHaveAttribute('aria-expanded', 'false')
      await count.click()
      await expect(summary).toHaveAttribute('aria-expanded', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('count badge sits between chevron and label inside the summary', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeOrder')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const summary = board.columnToggle('Ready')
      await expect(summary).toBeVisible()
      const order = await summary.evaluate((el) => Array.from(el.children).map((c) => c.className))
      expect(order[0]).toContain('board-group-chevron')
      expect(order[1]).toContain('board-group-count')
      expect(order[2]).toContain('board-group-label')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('no console errors on count render and toggle cycle', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeNoErrors')
      await win.reload()
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const count = board.columnCount('Ready')
      await count.click()
      await count.click()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('#467 board panel — bounded scroll + sticky headers + native <details>', () => {
  test('section toggling uses native <details>/<summary> primitives', async () => {
    const { app, win, mockGh } = await launchWithMockGhAndSeededIteration({
      name: 'ScrollStickyDetails'
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(async () => {
        const detailsCount = await win.locator('details.board-group').count()
        expect(detailsCount).toBeGreaterThan(0)
        const summaryCount = await win
          .locator('details.board-group > summary.board-group-summary')
          .count()
        expect(summaryCount).toBe(detailsCount)
      }).toPass()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('default browser disclosure-triangle is suppressed', async () => {
    const { app, win, mockGh } = await launchWithMockGhAndSeededIteration({
      name: 'ScrollStickyDisclosure'
    })
    try {
      const summary = new BoardPage(win).columnToggle('Ready')
      const listStyle = await summary.evaluate((el) => getComputedStyle(el).listStyleType)
      expect(listStyle).toBe('none')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('multi-expand: opening one section does not auto-collapse others', async () => {
    const { app, win, mockGh } = await launchWithMockGhAndSeededIteration({
      name: 'ScrollStickyMultiExpand'
    })
    try {
      const ready = new BoardPage(win).columnToggle('Ready')
      const inProgress = new BoardPage(win).columnToggle('In Progress')
      const accepted = new BoardPage(win).columnToggle('Accepted')

      await expect(ready).toHaveAttribute('aria-expanded', 'true')
      await expect(inProgress).toHaveAttribute('aria-expanded', 'true')

      await new BoardPage(win).openTab('Done')
      await expect(accepted).toHaveAttribute('aria-expanded', 'false')

      await accepted.click()
      await expect(accepted).toHaveAttribute('aria-expanded', 'true')

      await new BoardPage(win).openTab('Now')
      await expect(ready).toHaveAttribute('aria-expanded', 'true')
      await expect(inProgress).toHaveAttribute('aria-expanded', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('board-body has bounded vertical scroll affordance', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(new BoardPage(win).region()).toBeVisible()

      const body = win.locator('.board-body')
      const overflowY = await body.evaluate((el) => getComputedStyle(el).overflowY)
      expect(overflowY).toBe('auto')

      const minHeight = await body.evaluate((el) => getComputedStyle(el).minHeight)
      expect(minHeight).toBe('0px')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('section headers are sticky-positioned with opaque background', async () => {
    const { app, win, mockGh } = await launchWithMockGhAndSeededIteration({
      name: 'ScrollStickySticky'
    })
    try {
      const summary = new BoardPage(win).columnToggle('Ready')
      const position = await summary.evaluate((el) => getComputedStyle(el).position)
      expect(position).toBe('sticky')

      const top = await summary.evaluate((el) => getComputedStyle(el).top)
      expect(top).toBe('0px')

      const bg = await summary.evaluate((el) => getComputedStyle(el).backgroundColor)
      expect(bg).not.toBe('rgba(0, 0, 0, 0)')
      expect(bg).not.toBe('transparent')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('chevron rotates between collapsed and expanded via [open] selector', async () => {
    const { app, win, mockGh } = await launchWithMockGhAndSeededIteration({
      name: 'ScrollStickyChevron'
    })
    try {
      const summary = new BoardPage(win).columnToggle('Ready')
      const chevron = summary.locator('.board-group-chevron')
      const before = (): Promise<string> =>
        chevron.evaluate((el) => getComputedStyle(el, '::before').content)

      expect(await before()).toBe('"▾"')
      await summary.click()
      expect(await before()).toBe('"▸"')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('no console errors or hydration warnings on expand/collapse cycle', async () => {
    const { app, win, mockGh } = await launchWithMockGhAndSeededIteration({
      name: 'ScrollStickyNoErrors'
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      consoleErrors.length = 0
      pageErrors.length = 0

      const summary = new BoardPage(win).columnToggle('Ready')
      await summary.click()
      await summary.click()
      await summary.click()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — persistence-class — board collapse state', () => {
  test('user-toggled icebox collapse survives window reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.openTab('Plan')

      const iceboxToggle = board.columnToggle('Icebox')
      await expect(iceboxToggle).toHaveAttribute('aria-expanded', 'true')

      await iceboxToggle.click()
      await expect(iceboxToggle).toHaveAttribute('aria-expanded', 'false')

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')

      const iceboxAfterReload = board.columnToggle('Icebox')
      await expect(iceboxAfterReload).toHaveAttribute('aria-expanded', 'false')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('user-toggled in-progress collapse survives window reload', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await seedCurrentIterationViaRpc(rpc, 'HumanBoardCollapseInProgress')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const inProgressToggle = board.columnToggle('In Progress')
      await expect(inProgressToggle).toHaveAttribute('aria-expanded', 'true')

      await inProgressToggle.click()
      await expect(inProgressToggle).toHaveAttribute('aria-expanded', 'false')

      await win.reload()
      await expect(board.region()).toBeVisible()

      const inProgressAfterReload = board.columnToggle('In Progress')
      await expect(inProgressAfterReload).toHaveAttribute('aria-expanded', 'false')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('#505 slice 5c — Board panel DESK-CHECK column', () => {
  test('column renders between IN PROGRESS and DELIVERED', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'HumanBoardDeskCheckCol-1')
      await win.reload()
      await waitForBoardLoaded(win)

      await expect
        .poll(() =>
          board
            .nowContent()
            .getByRole('group')
            .evaluateAll((els) => els.map((el) => el.getAttribute('aria-label')))
        )
        .toEqual(['Ready', 'In Progress', 'Desk Check', 'Acceptance'])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('DESK-CHECK column renders empty-state with (0) count when no stories present', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'HumanBoardDeskCheckCol-2')
      await win.reload()
      await waitForBoardLoaded(win)

      await expect(board.group('Desk Check')).toBeVisible()
      await expect(board.columnCount('Desk Check')).toHaveText('(0)')
      await expect(board.columnCount('Desk Check')).toHaveAttribute('data-empty', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('DESK-CHECK column header has matching styling to other columns', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'HumanBoardDeskCheckCol-3')
      await win.mouse.move(0, 0)
      await win.reload()
      await win.mouse.move(0, 0)
      await waitForBoardLoaded(win)

      const deskCheck = board.columnToggle('Desk Check')
      const inProgress = board.columnToggle('In Progress')

      const deskCheckPosition = await deskCheck.evaluate((el) => getComputedStyle(el).position)
      const inProgressPosition = await inProgress.evaluate((el) => getComputedStyle(el).position)
      expect(deskCheckPosition).toBe(inProgressPosition)

      const deskCheckBg = await deskCheck.evaluate((el) => getComputedStyle(el).backgroundColor)
      const inProgressBg = await inProgress.evaluate((el) => getComputedStyle(el).backgroundColor)
      expect(deskCheckBg).toBe(inProgressBg)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('no console errors when DESK-CHECK column renders empty', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'HumanBoardDeskCheckCol-4')
      await win.reload()
      await waitForBoardLoaded(win)
      await expect(board.group('Desk Check')).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

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
