import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'

// #1127 sub-slice 1: pattern-establish migration off BPMUX_TEST_MODE +
// /api/test/<gate> onto httpmock-wrapped GitHubSourceAdapter +
// /api/rpc/<tool>. Same single-gate user-job — human watches a story
// leave IN-PROGRESS visibly after deliver_story — but driven through
// the production code path bpmux uses for real agents. File renamed
// from `human-observes-delivered-state-after-deliver-story` per the
// post-#516 ceremony: deliver_story now lands in DESK-CHECK column
// (designer's pass_desk_check then routes to DELIVERED, covered by
// a sibling spec).

test.describe('human-side E2E #476 — observation-class — deliver_story', () => {
  test('human observes IN-PROGRESS then DESK-CHECK across live-mutation-chain', async () => {
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
        'applications/electron/tests/human-observes-desk-check-state-after-deliver-story.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] human-observes-delivered',
          user_job:
            'When dev delivers a story, human needs to see it in the DELIVERED column on Electron without manual refresh-thinking.',
          observation_falsifier:
            'reject if board does not show story in DELIVERED column after deliver_story round-trip',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'HumanObservesDeskCheck')

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase:
          'Picking up the human-observes-delivered story to verify the live-mutation-chain.',
        falsifier_reframing:
          'reject if start_story does not move the story into IN-PROGRESS visible to the human'
      })
      expect(startResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.storyInColumn('In Progress', 1)).toHaveCount(1)
      await expect(board.storyInColumn('In Progress', 1)).toHaveAttribute(
        'aria-label',
        /human-observes-delivered/
      )

      const deliverResp = await rpc('deliver_story', {
        id: 1,
        verified_observation:
          'Drove deliver via /api/rpc/deliver_story from Playwright after observing IN-PROGRESS; verifying DESK-CHECK render through real bridge → BoardStore → /api/board.'
      })
      expect(deliverResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.storyInColumn('Desk Check', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Desk Check', 1)).toHaveAttribute(
        'aria-label',
        /human-observes-delivered/
      )

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
