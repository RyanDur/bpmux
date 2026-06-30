import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

// #1127 sub-slice 3: owner-specific actions visible on a story that
// reached DELIVERED column. Setup: create→start→deliver→pass_desk_check.

test.describe('human-side E2E #476 — owner-specific — accept/reject buttons on delivered drill-in', () => {
  test('owner sees accept + reject buttons in drill-in actions for a delivered story', async () => {
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
        'applications/electron/tests/human-owner-action-buttons-on-delivered-drillin.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] owner-actions-target',
          user_job:
            'When the owner opens a delivered story drill-in, accept + reject buttons should be visible.',
          observation_falsifier:
            'reject if owner-action buttons are absent on a delivered drill-in',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'OwnerActions')

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase: 'Picking up to drive deliver before owner-walk.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        id: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check next.'
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

      await expect(board.storyInColumn('Acceptance', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Acceptance', 1)).toHaveAttribute(
        'aria-label',
        /owner-actions-target/
      )

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.openStory(1)

      const drill = new DrillInPage(win)
      await expect(drill.panel()).toBeVisible()
      await expect(drill.acceptButton()).toBeVisible()
      await expect(drill.rejectButton()).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
