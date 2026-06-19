import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { addStoryToIterationViaRpc, seedCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

test.describe('human-side E2E #476 — direct-action — board keyboard navigation', () => {
  test('board is a roving-tabindex composite: one tab-stop, arrows rove, native-button Enter opens', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await waitForBoardLoaded(win)

      for (const title of ['[Feature] kbd-nav-first', '[Feature] kbd-nav-second']) {
        const createResp = await rpc(
          'create_story',
          {
            title,
            user_job:
              'When the human keyboard-navigates the board, arrows rove and Enter opens the focused row.',
            observation_falsifier:
              'reject if board is not one tab-stop OR arrows do not rove OR native Enter does not open drill-in',
            acceptance_test_path:
              'applications/electron/tests/human-board-keyboard-navigation.spec.ts'
          },
          'pm'
        )
        expect(createResp.status).toBe(200)
      }
      for (const number of [1, 2]) {
        const moveResp = await rpc(
          'move_story',
          { number, target_state: 'ready', rationale: 'kbd-nav roving fixture' },
          'pm'
        )
        expect(moveResp.status).toBe(200)
      }

      const iterNumber = await seedCurrentIterationViaRpc(rpc, 'KbdNavRoving')
      await addStoryToIterationViaRpc(rpc, 1, iterNumber, 1)
      await addStoryToIterationViaRpc(rpc, 2, iterNumber, 2)

      await win.reload()
      await waitForBoardLoaded(win)

      const board = new BoardPage(win)
      const navOf = (n: number): ReturnType<typeof win.locator> => board.storyNavigate(n)
      const oneTabStop = board.nowContent().locator('.board-story-navigate[tabindex="0"]')

      await expect(board.storyNavigate(1)).toHaveAttribute('aria-label', /kbd-nav-first/)

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(navOf(1)).toHaveAttribute('tabindex', '0')
      await expect(navOf(2)).toHaveAttribute('tabindex', '-1')
      await expect(oneTabStop).toHaveCount(1)

      await navOf(1).focus()
      await win.keyboard.press('ArrowDown')

      await expect(navOf(2)).toBeFocused()
      await expect(navOf(2)).toHaveAttribute('tabindex', '0')
      await expect(navOf(1)).toHaveAttribute('tabindex', '-1')
      await expect(board.storyRow(2)).toHaveAttribute('aria-current', 'true')
      await expect(oneTabStop).toHaveCount(1)

      await win.keyboard.press('Enter')
      const drill = new DrillInPage(win)
      await expect(drill.panel()).toBeVisible()
      await expect(drill.panel()).toContainText('kbd-nav-second')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
