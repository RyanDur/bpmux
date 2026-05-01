import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// #1127 sub-slice 2: replicate human-observes-* migration onto the
// close_story sibling. Setup walks create→start so the story is in
// IN-PROGRESS (any non-icebox active-pipeline state); single-gate
// under test = close_story; assertion = ACCEPTED column. columnFor
// (renderer/src/board/types.ts:36) routes closed+non-icebox →
// accepted; closed+icebox legitimately STAYS in icebox per #464's
// substrate decoder (preserved-as-historical). Pre-#1127 spec
// closed an icebox chore and asserted ACCEPTED; that's wrong-shape
// for the chore+icebox path. Driving create→start→close lands the
// canonical "PM closes mid-pipeline filing → vanishes from active
// queue" user-job at ACCEPTED column.

test.describe('human-side E2E #476 — observation-class — close_story', () => {
  test('human observes ACCEPTED after close_story across live-mutation-chain via close-mechanism', async () => {
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
        'applications/electron/tests/human-observes-accepted-state-after-close-story.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] human-observes-closed',
          user_job:
            'When PM closes a mid-pipeline filing (e.g., superseded by a sister), human needs to see it leave the active queue and land in ACCEPTED so the column counts settle.',
          observation_falsifier:
            'reject if board does not show story in ACCEPTED column after close_story round-trip',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase:
          'Pickup so the story leaves icebox; close from a mid-pipeline state exercises the canonical close_story path.',
        falsifier_reframing:
          'reject if close_story does not move the story into ACCEPTED column visible to the human'
      })
      expect(startResp.status).toBe(200)

      // start_story enqueues mutate_sync events but DOESN'T await
      // drain_pending — labels reach GH on the next drain_loop tick
      // (BPMUX_POLL_INTERVAL_SECS=1 set in launchWithMockGh). Without
      // this wait, close_story's refresh_story would re-fetch from
      // mock GH while it still shows the create-time `icebox` label,
      // overwriting the in-progress state we just set in BoardStore.
      await win.waitForTimeout(1500)

      const closeResp = await rpc(
        'close_story',
        {
          number: 1,
          reason: 'superseded by a sister filing — closing without accept-walk'
        },
        'pm'
      )
      expect(closeResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      await win.locator('[data-test="board-group-accepted"]').click()
      const storyAccepted = win.locator(
        '#board-group-accepted-content [data-test="board-story-1"]'
      )
      await expect(storyAccepted).toContainText('human-observes-closed')

      for (const otherState of ['ready', 'in-progress', 'desk-check', 'delivered', 'icebox']) {
        const storyElsewhere = win.locator(
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
