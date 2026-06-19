import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

// #1127 sub-slice 3: owner-action buttons must NOT surface on
// non-delivered stories. Setup: create only (story stays in icebox).

test.describe('human-side E2E #476 — owner-specific — actions hidden on non-delivered drill-in', () => {
  test('owner does NOT see accept/reject on icebox story drill-in', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] icebox-not-actionable',
          user_job:
            'When the owner opens an icebox story, accept/reject buttons should NOT be present (story is not delivered yet).',
          observation_falsifier:
            'reject if accept/reject buttons appear on a non-delivered story drill-in',
          acceptance_test_path:
            'applications/electron/tests/human-owner-actions-hidden-on-non-delivered.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')

      await expect(board.storyRow(1)).toContainText('icebox-not-actionable')

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.storyNavigate(1).click()

      await expect(drillIn.panel()).toBeVisible()

      await expect(drillIn.acceptButton()).toHaveCount(0)
      await expect(drillIn.rejectButton()).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
