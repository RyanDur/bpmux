import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('human-side E2E #476/#1377 — observation-class — close_story', () => {
  test('human observes RETIRED after close_story with retire-class reason', async () => {
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

      const acceptanceTestPath =
        'applications/electron/tests/human-observes-retired-state-after-close-story.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] human-observes-closed',
          user_job:
            'When PM closes a mid-pipeline filing as superseded, human needs to see it route to the RETIRED column (not ACCEPTED) so the success narrative is preserved.',
          observation_falsifier:
            'reject if board does not show story in RETIRED column after close_story with retire-class reason',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase:
          'Pickup so the story leaves icebox; close from a mid-pipeline state exercises the canonical close_story path.',
        falsifier_reframing:
          'reject if close_story does not move the story into RETIRED column visible to the human'
      })
      expect(startResp.status).toBe(200)

      await win.waitForTimeout(1500)

      const closeResp = await rpc(
        'close_story',
        {
          id: 1,
          reason: 'superseded by a sister filing — closing without accept-walk',
          disposition: 'retire'
        },
        'pm'
      )
      expect(closeResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Done')
      await expect(board.storyInColumn('Retired', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Retired', 1)).toHaveAttribute(
        'aria-label',
        /human-observes-closed/
      )

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
