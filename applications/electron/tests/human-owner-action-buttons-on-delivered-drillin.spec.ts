import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'

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

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

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

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Picking up to drive deliver before owner-walk.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check next.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc('pass_desk_check', { number: 1, would_file_bug: false }, 'designer')
      expect(passResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const storyRow = win.locator(
        '#board-group-delivered-content [data-test="board-story-1"]'
      )
      await expect(storyRow).toContainText('owner-actions-target')

      consoleErrors.length = 0
      pageErrors.length = 0

      await storyRow.locator('.board-story-button').click()

      const drillIn = win.locator('[data-test="board-drill-in"]')
      await expect(drillIn).toBeVisible()

      const actions = win.locator('[data-test="drill-in-actions"]')
      await expect(actions).toBeVisible()
      await expect(win.locator('[data-test="drill-in-accept"]')).toBeVisible()
      await expect(win.locator('[data-test="drill-in-reject"]')).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
