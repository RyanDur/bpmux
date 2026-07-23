import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('human-side E2E #476 — live-update class — BOARD_INVALIDATE refresh', () => {
  test('story created via rpc appears WITHOUT manual reload (live WS-driven refetch)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await waitForBoardLoaded(win)

      consoleErrors.length = 0
      pageErrors.length = 0

      const board = new BoardPage(win)
      const initial = board.storyAnywhere(1)
      await expect(initial).toHaveCount(0)

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] live-board-invalidate-marker',
          user_job:
            'When a peer creates a story, the human watching the Electron board needs the new story to appear live without manual refresh.',
          observation_falsifier:
            'reject if board does not show the new story after BOARD_INVALIDATE WS broadcast',
          acceptance_test_path: atpPath(__filename)
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await expect(board.storyAnywhere(1).first()).toHaveAttribute(
        'aria-label',
        /live-board-invalidate-marker/,
        { timeout: 10000 }
      )

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
