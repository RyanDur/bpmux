import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'

test.describe('#1720 — board reflects a real state-transition LIVE (no reload)', () => {
  test('a peer-driven transition moves the rendered card to its new column without any manual reload', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await waitForBoardLoaded(win)

      const createResp = await rpc(
        'create_story',
        {
          title: '[Bug] live-transition-marker',
          user_job:
            'When a peer transitions a story, the human watching the Electron board needs the card to move to its new column live, without a manual reload.',
          observation_falsifier:
            'reject if the card stays in its prior column after a real transition until a manual reload',
          acceptance_test_path:
            'applications/electron/tests/human-1720-board-live-transition.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'Live1720Transition')

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Picking up the live-transition board-freshness story for verification.',
        falsifier_reframing:
          'reject if a real transition does not move the rendered card to its new column live.'
      })
      expect(startResp.status).toBe(200)

      const inProgressCard = win.locator(
        '#board-group-in-progress-content [data-test="board-story-1"]'
      )
      await expect(inProgressCard).toContainText('live-transition-marker', { timeout: 10000 })

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation:
          'Drove deliver via /api/rpc/deliver_story to stage the desk-check transition.'
      })
      expect(deliverResp.status).toBe(200)

      const deskCheckCard = win.locator(
        '#board-group-desk-check-content [data-test="board-story-1"]'
      )
      await expect(deskCheckCard).toContainText('live-transition-marker', { timeout: 10000 })
      await expect(inProgressCard).toHaveCount(0)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc(
        'pass_desk_check',
        { number: 1, acceptor: 'owner', would_file_bug: false },
        'designer'
      )
      expect(passResp.status).toBe(200)

      const deliveredCard = win.locator(
        '#board-group-delivered-content [data-test="board-story-1"]'
      )
      await expect(deliveredCard).toContainText('live-transition-marker', { timeout: 10000 })
      await expect(deskCheckCard).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
