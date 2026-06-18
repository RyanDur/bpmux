import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

// #1127 sub-slice 3: drill-in family — comment send button gating.

test.describe('human-side E2E #476 — direct-action — drill-in comment input', () => {
  test('comment send button is disabled when input is empty, enabled when typed', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drill = new DrillInPage(win)
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] comment-input-target',
          user_job:
            'When the human opens a drill-in, the comment send button should be disabled until they type something.',
          observation_falsifier:
            'reject if comment send button is enabled with empty input OR remains disabled after typing',
          acceptance_test_path:
            'applications/electron/tests/human-drill-in-comment-input-disabled-when-empty.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.storyNavigate(1)).toHaveAccessibleName(/comment-input-target/)

      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const sendButton = drill.commentSend()
      await expect(sendButton).toBeVisible()
      await expect(sendButton).toBeDisabled()

      const commentInput = drill.commentInput()
      await commentInput.fill('Some non-trivial comment text.')
      await expect(sendButton).toBeEnabled()

      await commentInput.fill('')
      await expect(sendButton).toBeDisabled()

      await commentInput.fill('   ')
      await expect(sendButton).toBeDisabled()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
