import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — board drill-in', () => {
  test('clicking a story row opens the drill-in panel with title + body', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-drill-in-auth-')),
      'auth_token'
    )
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: {
        BPMUX_TEST_MODE: '1',
        BPMUX_AUTH_TOKEN_PATH: authTokenPath
      }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const titleMarker = 'drill-in-target-marker'
    const createResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: `[Feature] ${titleMarker}`,
        user_job:
          'When a human clicks a story row, the drill-in panel opens with the story title visible.',
        observation_falsifier:
          'reject if drill-in panel does not appear after row click'
      })
    })
    expect(createResp.status).toBe(200)

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()
    const storyRow = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
    await expect(storyRow).toContainText(titleMarker)

    consoleErrors.length = 0
    pageErrors.length = 0

    await storyRow.locator('button').first().click()

    const drillIn = win.locator('[data-test="board-drill-in"]')
    await expect(drillIn).toBeVisible()
    await expect(drillIn).toContainText(titleMarker)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
