import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — board keyboard navigation', () => {
  test('ArrowDown selects first story; Enter opens drill-in', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-board-kbd-auth-')),
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

    await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: '[Feature] kbd-nav-target',
        user_job: 'When the human focuses the board, ArrowDown navigates to the first story.',
        observation_falsifier:
          'reject if ArrowDown does not select a story OR Enter does not open drill-in'
      })
    }).then((r) => expect(r.status).toBe(200))

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

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

    await app.close()
  })
})
