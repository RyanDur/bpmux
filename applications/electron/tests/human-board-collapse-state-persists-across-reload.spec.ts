import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

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
