import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// #1127 sub-slice 3: drill-in family — body content rendering.

test.describe('human-side E2E #476 — read-class — drill-in body content', () => {
  test('drill-in renders the story user_job + observation_falsifier markers from the body', async () => {
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

      const userJobMarker = 'USER_JOB_DRILL_IN_BODY_BPF12'
      const falsifierMarker = 'FALSIFIER_DRILL_IN_BODY_BPF12'

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] drill-body-target',
          user_job: userJobMarker,
          observation_falsifier: falsifierMarker,
          acceptance_test_path:
            'applications/electron/tests/human-drill-in-shows-story-body-and-userjob.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const storyRow = win.locator('[data-test="board-story-1"]').first()
      await expect(storyRow).toContainText('drill-body-target')

      await storyRow.locator('button').first().click()
      const drillIn = win.locator('[data-test="board-drill-in"]')
      await expect(drillIn).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(drillIn).toContainText(userJobMarker, { timeout: 10000 })
      await expect(drillIn).toContainText(falsifierMarker)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
