import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

// #1127 sub-slice 3: story-badge rendering on a story that reached
// DELIVERED column (post-pass_desk_check; the awaiting-owner badge
// derives from state=delivered + Feature story_type).

test.describe('human-side E2E #476 — read-class — story badge', () => {
  test('delivered Feature renders the awaiting-owner badge', async () => {
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

      const acceptanceTestPath = atpPath(__filename)
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] badge-render-target',
          user_job:
            'When dev delivers a Feature story, human needs to see the awaiting-owner badge on the row.',
          observation_falsifier:
            'reject if delivered Feature does not render the awaiting-owner badge variant',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'BadgeRenderDelivered')

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase: 'Picking up to drive the badge-render assertion.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        id: 1,
        verified_observation: 'Drove the chain to deliver so designer can pass desk-check.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc(
        'pass_desk_check',
        { id: 1, acceptor: 'owner', would_file_bug: false },
        'designer'
      )
      expect(passResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.storyInColumn('Acceptance', 1)).toHaveAttribute(
        'aria-label',
        /badge-render-target/
      )

      consoleErrors.length = 0
      pageErrors.length = 0

      const badge = board.storyBadge('Acceptance', 'awaiting owner')
      await expect(badge).toBeVisible()
      await expect(badge).toHaveAttribute('data-badge-kind', 'accepted')
      await expect(badge).toHaveAttribute('data-state', 'acceptance')
      await expect(badge).toContainText('awaiting owner')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
