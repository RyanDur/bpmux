import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — observation-class — close_story', () => {
  test('human observes ICEBOX then ACCEPTED across live-mutation-chain via close-mechanism', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-close-auth-')),
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

    const createResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: '[Chore] human-observes-closed',
        user_job:
          'When PM closes a chore without an accept-walk (e.g., redundant filing), human needs to see it in the ACCEPTED column so the queue settles.',
        observation_falsifier:
          'reject if board does not show story in ACCEPTED column after close_story round-trip'
      })
    })
    expect(createResp.status).toBe(200)

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()
    const storyInIcebox = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
    await expect(storyInIcebox).toContainText('human-observes-closed')

    const closeResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/close_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        reason: 'redundant filing — closing without accept-walk'
      })
    })
    expect(closeResp.status).toBe(200)

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-accepted"]').click()
    const storyAccepted = win.locator(
      '#board-group-accepted-content [data-test="board-story-1"]'
    )
    await expect(storyAccepted).toContainText('human-observes-closed')

    for (const otherState of ['ready', 'in-progress', 'delivered', 'icebox']) {
      const storyElsewhere = win.locator(
        `#board-group-${otherState}-content [data-test="board-story-1"]`
      )
      await expect(storyElsewhere).toHaveCount(0)
    }

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
