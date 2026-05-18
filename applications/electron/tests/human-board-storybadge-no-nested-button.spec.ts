import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'

// #1127 sub-slice 3 + #499 substrate: StoryBadge with a click handler
// renders as <button>; pre-#499 StoryRow's outer element was also
// <button>, producing nested-interactive HTML + hydration warnings.
// Fix-direct: StoryRow uses <div role="button" tabIndex={0}> with
// Enter/Space keydown activation. Spec exercises a delivered Feature
// (which renders StoryBadge with onOpenTimeline handler) and asserts:
// (1) board renders without console errors; (2) row click opens
// drill-in; (3) keyboard Enter on focused row also opens drill-in.

test.describe('human-side E2E #499 — StoryBadge no nested-button', () => {
  test('delivered story renders + opens via click + keyboard without hydration warnings', async () => {
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

      const acceptanceTestPath =
        'applications/electron/tests/human-board-storybadge-no-nested-button.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] storybadge-nesting-target',
          user_job:
            'When the human looks at a delivered Feature, the StoryBadge surfaces an awaiting-owner indicator without nested-button hydration warnings.',
          observation_falsifier:
            'reject if board emits console errors or hydration warnings on a delivered Feature row',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Driving to deliver so badge surfaces with handler.',
        falsifier_reframing: 'reject if start does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc('pass_desk_check', { number: 1, would_file_bug: false }, 'designer')
      expect(passResp.status).toBe(200)

      await win.reload()
      await waitForBoardLoaded(win)

      const row = win.locator('#board-group-delivered-content [data-test="board-story-1"]')
      await expect(row).toContainText('storybadge-nesting-target')

      const badge = row.locator('[data-test="story-badge"]').first()
      await expect(badge).toBeVisible()

      await row.click()
      const drillIn = win.locator('[data-test="board-drill-in"]')
      await expect(drillIn).toBeVisible()
      await win.keyboard.press('Escape')
      await expect(drillIn).not.toBeVisible({ timeout: 5000 })

      const button = row.locator('.board-story-button')
      await button.focus()
      await win.keyboard.press('Enter')
      await expect(drillIn).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
