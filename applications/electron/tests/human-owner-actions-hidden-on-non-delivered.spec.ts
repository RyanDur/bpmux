import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// #1127 sub-slice 3: owner-action buttons must NOT surface on
// non-delivered stories. Setup: create only (story stays in icebox).

test.describe('human-side E2E #476 — owner-specific — actions hidden on non-delivered drill-in', () => {
  test('owner does NOT see accept/reject on icebox story drill-in', async () => {
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

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] icebox-not-actionable',
          user_job:
            'When the owner opens an icebox story, accept/reject buttons should NOT be present (story is not delivered yet).',
          observation_falsifier:
            'reject if accept/reject buttons appear on a non-delivered story drill-in',
          acceptance_test_path:
            'applications/electron/tests/human-owner-actions-hidden-on-non-delivered.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const storyRow = win.locator('[data-test="board-story-1"]').first()
      await expect(storyRow).toContainText('icebox-not-actionable')

      consoleErrors.length = 0
      pageErrors.length = 0

      await storyRow.locator('button').first().click()

      const drillIn = win.locator('[data-test="board-drill-in"]')
      await expect(drillIn).toBeVisible()

      await expect(win.locator('[data-test="drill-in-accept"]')).toHaveCount(0)
      await expect(win.locator('[data-test="drill-in-reject"]')).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
