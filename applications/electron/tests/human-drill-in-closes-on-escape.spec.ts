import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// #1127 sub-slice 3: drill-in family — Escape-key closes drill-in.

test.describe('human-side E2E #476 — direct-action — drill-in close', () => {
  test('Escape closes the drill-in panel and returns to board', async () => {
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
          title: '[Feature] drill-close-target',
          user_job: 'When the human presses Escape in a drill-in, the drill-in closes.',
          observation_falsifier: 'reject if Escape does not close the drill-in panel',
          acceptance_test_path:
            'applications/electron/tests/human-drill-in-closes-on-escape.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      await win.locator('[data-test="board-group-icebox"]').click()
      const storyRow = win.locator(
        '#board-group-icebox-content [data-test="board-story-1"]'
      )
      await expect(storyRow).toContainText('drill-close-target')

      await storyRow.locator('.board-story-button').click()
      const drillIn = win.locator('[data-test="board-drill-in"]')
      await expect(drillIn).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.keyboard.press('Escape')
      await expect(drillIn).toHaveCount(0)
      await expect(win.locator('[data-test="board-list"]')).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
