import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — live-update class — BOARD_INVALIDATE refresh', () => {
  test('story created via test-route appears WITHOUT manual reload (live WS-driven refetch)', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-board-invalidate-live-auth-')),
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

    const initial = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
    await expect(initial).toHaveCount(0)

    const createResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: '[Feature] live-board-invalidate-marker',
        user_job:
          'When a peer creates a story, the human watching the Electron board needs the new story to appear live without manual refresh.',
        observation_falsifier:
          'reject if board does not show the new story after BOARD_INVALIDATE WS broadcast'
      })
    })
    expect(createResp.status).toBe(200)

    await win.locator('[data-test="board-group-icebox"]').click()
    const storyAfterInvalidate = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
    await expect(storyAfterInvalidate).toContainText('live-board-invalidate-marker', {
      timeout: 5000
    })

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
