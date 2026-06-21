import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { BoardPage } from './pages/boardPage'

// #1127 sub-slice 2: replicate human-observes-* migration onto the
// accept_story sibling. Setup walks create→start→deliver to get a
// story into DESK CHECK; single-gate under test = accept_story;
// assertion = ACCEPTED column. The pre-#516 spec also drove deliver
// + asserted DELIVERED column mid-run; that intermediate is no longer
// the canonical destination of deliver_story (now DESK CHECK) and
// belongs to its own sibling spec, so we drop it here to keep the
// family-pattern single-gate-per-assertion intact.

test.describe('human-side E2E #476 — observation-class — accept_story', () => {
  test('human observes ACCEPTED after accept_story across live-mutation-chain', async () => {
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
        'applications/electron/tests/human-observes-accepted-state-after-accept-story.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] human-observes-accepted',
          user_job:
            'When a delivered story is accepted, the human watching the board needs to see it move to the ACCEPTED column on Electron without manual refresh-thinking.',
          observation_falsifier:
            'reject if board does not show story in ACCEPTED column after accept_story round-trip',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase:
          'Picking up the human-observes-accepted story to verify the accept-transition.',
        falsifier_reframing:
          'reject if accept_story does not move the story into ACCEPTED visible to the human'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation:
          'Drove deliver via /api/rpc/deliver_story; queueing accept_story as the gate under test.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer', 'dev'])

      const labelResp = await rpc(
        'label_story',
        {
          number: 1,
          label: 'awaiting team',
          rationale: '#1460 test fixture: acceptor=team for team-verifiable accept'
        },
        'pm'
      )
      expect(labelResp.status).toBe(200)

      const acceptResp = await rpc('accept_story', { number: 1, would_file_bug: false }, 'pm')
      expect(acceptResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')

      await expect(board.storyInColumn('Accepted', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Accepted', 1)).toHaveAttribute(
        'aria-label',
        /human-observes-accepted/
      )

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
