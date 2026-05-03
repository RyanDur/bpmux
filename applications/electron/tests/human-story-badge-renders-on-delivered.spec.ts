import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

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

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const acceptanceTestPath =
        'applications/electron/tests/human-story-badge-renders-on-delivered.spec.ts'
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

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Picking up to drive the badge-render assertion.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation: 'Drove the chain to deliver so designer can pass desk-check.'
      })
      expect(deliverResp.status).toBe(200)

      const passResp = await rpc('pass_desk_check', { number: 1, would_file_bug: false }, 'designer')
      expect(passResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const storyRow = win.locator(
        '#board-group-delivered-content [data-test="board-story-1"]'
      )
      await expect(storyRow).toContainText('badge-render-target')

      consoleErrors.length = 0
      pageErrors.length = 0

      const badge = storyRow.locator('[data-test="story-badge"]')
      await expect(badge).toBeVisible()
      await expect(badge).toHaveAttribute('data-badge-kind', 'accepted')
      await expect(badge).toHaveAttribute('data-state', 'delivered')
      await expect(badge).toContainText('awaiting owner')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
