import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

// #1127 sub-slice 3: drill-in family migration. Setup: create_story
// only (story lands in ICEBOX). Gate under test: row-click → drill-in
// open. No backend gate-tool firing in the assertion phase; this
// exercises the renderer's drill-in handler against a real-substrate-
// backed board fetch.

test.describe('human-side E2E #476 — direct-action — board drill-in', () => {
  test('clicking a story row opens the drill-in panel with title + body', async () => {
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

      const titleMarker = 'drill-in-target-marker'
      const createResp = await rpc(
        'create_story',
        {
          title: `[Feature] ${titleMarker}`,
          user_job:
            'When a human clicks a story row, the drill-in panel opens with the story title visible.',
          observation_falsifier: 'reject if drill-in panel does not appear after row click',
          acceptance_test_path:
            'applications/electron/tests/human-drill-in-opens-on-story-click.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')

      await expect(board.storyNavigate(1)).toHaveAccessibleName(new RegExp(titleMarker))

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.openStory(1)

      await expect(drill.panel()).toBeVisible()
      await expect(drill.panel()).toContainText(titleMarker)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
