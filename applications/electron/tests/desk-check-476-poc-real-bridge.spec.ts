import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('desk-check #476 POC — real-bridge round-trip via /api/test/create_story', () => {
  test('Feature created via test-route appears on board after reload', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-poc-auth-')),
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

    const resp = await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: '[Feature] poc-real-bridge-roundtrip',
        user_job:
          'When testing E2E, dev needs the HTTP /api/test/create_story → /api/board round-trip to flow through the real bridge.',
        observation_falsifier:
          'reject if board does not show new story after reload, or if console.error/pageerror fires during the round-trip'
      })
    })
    expect(resp.status).toBe(200)
    const json = (await resp.json()) as { message: string }
    expect(json.message).toContain('story created:')

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()

    const storyRow = win.locator('[data-test="board-story-1"]')
    await expect(storyRow).toBeVisible()
    await expect(storyRow).toContainText('poc-real-bridge-roundtrip')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
