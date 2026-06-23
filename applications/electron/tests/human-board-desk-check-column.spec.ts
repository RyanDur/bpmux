import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { seedCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

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
