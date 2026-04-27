import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow, waitForBoardLoaded } from './helpers/launch'

// #499: StoryBadge with a click-handler renders as <button>; before
// the fix, StoryRow's outer element was also <button>, producing
// nested-interactive HTML + React DOM hydration warnings on every
// board render with badges-having-handlers. Fix-direct (Option B):
// StoryRow's outer element is now <div role="button" tabIndex={0}>
// with Enter/Space keydown activation. StoryBadge stays interactive.
//
// Spec exercises a delivered Feature (which renders StoryBadge with
// onOpenTimeline handler) and asserts: (1) board renders without
// console errors / page errors; (2) clicking the row opens the
// drill-in; (3) keyboard Enter on the focused row also opens the
// drill-in.

test.describe('human-side E2E #499 — StoryBadge no nested-button', () => {
  test('delivered story renders + opens via click + keyboard without hydration warnings', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await waitForBoardLoaded(win)

    consoleErrors.length = 0
    pageErrors.length = 0

    await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: '[Feature] storybadge-nesting-target',
        user_job:
          'When the human looks at a delivered Feature, the StoryBadge surfaces an awaiting-owner indicator without nested-button hydration warnings.',
        observation_falsifier:
          'reject if board emits console errors or hydration warnings on a delivered Feature row'
      })
    }).then((r) => expect(r.status).toBe(200))

    await fetch(`http://127.0.0.1:${backendPort}/api/test/start_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        pickup_paraphrase: 'Driving to deliver so badge surfaces with handler.',
        falsifier_reframing: 'reject if start does not move story to in-progress'
      })
    }).then((r) => expect(r.status).toBe(200))

    await fetch(`http://127.0.0.1:${backendPort}/api/test/deliver_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        verified_observation: 'Drove to DELIVERED so the StoryBadge handler is present.'
      })
    }).then((r) => expect(r.status).toBe(200))

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

    await app.close()
  })
})
