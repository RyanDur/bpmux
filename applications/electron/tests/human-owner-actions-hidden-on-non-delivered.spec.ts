import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — owner-specific — actions hidden on non-delivered drill-in', () => {
  test('owner does NOT see accept/reject on icebox story drill-in', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-actions-hidden-auth-')),
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
        title: '[Feature] icebox-not-actionable',
        user_job:
          'When the owner opens an icebox story, accept/reject buttons should NOT be present (story is not delivered yet).',
        observation_falsifier:
          'reject if accept/reject buttons appear on a non-delivered story drill-in'
      })
    }).then((r) => expect(r.status).toBe(200))

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()
    const storyRow = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
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

    await app.close()
  })
})
