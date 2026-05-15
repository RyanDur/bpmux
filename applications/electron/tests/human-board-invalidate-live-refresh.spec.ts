import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'

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

      const initial = win.locator('[data-test="board-story-1"]')
      await expect(initial).toHaveCount(0)

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] live-board-invalidate-marker',
          user_job:
            'When a peer creates a story, the human watching the Electron board needs the new story to appear live without manual refresh.',
          observation_falsifier:
            'reject if board does not show the new story after BOARD_INVALIDATE WS broadcast',
          acceptance_test_path:
            'applications/electron/tests/human-board-invalidate-live-refresh.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      const storyAfterInvalidate = win.locator('[data-test="board-story-1"]').first()
      await expect(storyAfterInvalidate).toContainText('live-board-invalidate-marker', {
        timeout: 10000
      })

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
