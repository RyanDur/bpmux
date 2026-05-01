import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — board keyboard navigation', () => {
  test('ArrowDown selects first story; Enter opens drill-in', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await waitForBoardLoaded(win)

      consoleErrors.length = 0
      pageErrors.length = 0

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] kbd-nav-target',
          user_job: 'When the human focuses the board, ArrowDown navigates to the first story.',
          observation_falsifier:
            'reject if ArrowDown does not select a story OR Enter does not open drill-in',
          acceptance_test_path:
            'applications/electron/tests/human-board-keyboard-navigation.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await waitForBoardLoaded(win)

      await win.locator('[data-test="board-group-icebox"]').click()
      const storyRow = win.locator(
        '#board-group-icebox-content [data-test="board-story-1"]'
      )
      await expect(storyRow).toContainText('kbd-nav-target')

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.locator('[data-test="board-region"]').focus()

      await win.keyboard.press('ArrowDown')
      await expect(storyRow).toHaveAttribute('aria-current', 'true')

      await win.keyboard.press('Enter')
      const drillIn = win.locator('[data-test="board-drill-in"]')
      await expect(drillIn).toBeVisible()
      await expect(drillIn).toContainText('kbd-nav-target')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
