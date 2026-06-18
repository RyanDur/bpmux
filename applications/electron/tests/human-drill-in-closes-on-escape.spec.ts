import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

// #1127 sub-slice 3: drill-in family — Escape-key closes drill-in.

test.describe('human-side E2E #476 — direct-action — drill-in close', () => {
  test('Escape closes the drill-in panel and returns to board', async () => {
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
          title: '[Feature] drill-close-target',
          user_job: 'When the human presses Escape in a drill-in, the drill-in closes.',
          observation_falsifier: 'reject if Escape does not close the drill-in panel',
          acceptance_test_path:
            'applications/electron/tests/human-drill-in-closes-on-escape.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.storyNavigate(1)).toHaveAccessibleName(/drill-close-target/)

      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.keyboard.press('Escape')
      await expect(drill.panel()).toHaveCount(0)
      await expect(board.storyNavigate(1)).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
