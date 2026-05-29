import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'

// #1127 sub-slice 2: replicate human-observes-* migration onto the
// fail_desk_check sibling. Setup walks create→start→deliver to land
// the story in DESK CHECK; single-gate under test = fail_desk_check;
// assertion = IN-PROGRESS column. fail_desk_check transitions the
// story back to NEEDS-FIXES, which `columnFor` (renderer/src/board/
// types.ts:42) maps to the in-progress column for the user-facing
// pipeline view (rework returns to dev's lane visually).

test.describe('human-side E2E #476 — observation-class — fail_desk_check', () => {
  test('human observes IN-PROGRESS (needs-fixes) after fail_desk_check across live-mutation-chain', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const acceptanceTestPath =
        'applications/electron/tests/human-observes-needs-fixes-after-fail-desk-check.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] human-observes-needs-fixes',
          user_job:
            'When designer fails a delivered story’s desk-check, human needs to see it move back to IN-PROGRESS so dev knows it’s in rework.',
          observation_falsifier:
            'reject if board does not show story back in IN-PROGRESS column after fail_desk_check round-trip',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'NeedsFixesObserve')

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Picking up to verify the fail-desk-check rework return.',
        falsifier_reframing:
          'reject if fail_desk_check does not move the story back to NEEDS-FIXES (in-progress column) visible to the human'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation:
          'Drove deliver via /api/rpc/deliver_story; queueing fail_desk_check as the gate under test.'
      })
      expect(deliverResp.status).toBe(200)

      const failResp = await rpc(
        'fail_desk_check',
        {
          number: 1,
          reason: 'designer flagged accessibility regression on the touched surface'
        },
        'designer'
      )
      expect(failResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const nowPanel = win.locator('[data-test="board-tabpanel-now-content"]')
      const storyInProgress = nowPanel.locator(
        '#board-group-in-progress-content [data-test="board-story-1"]'
      )
      await expect(storyInProgress).toContainText('human-observes-needs-fixes')

      for (const otherState of ['ready', 'desk-check', 'delivered']) {
        const storyElsewhere = nowPanel.locator(
          `#board-group-${otherState}-content [data-test="board-story-1"]`
        )
        await expect(storyElsewhere).toHaveCount(0)
      }

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
