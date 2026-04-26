import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — observation-class — fail_desk_check', () => {
  test('human observes DELIVERED then IN-PROGRESS across live-mutation-chain', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-fail-desk-check-auth-')),
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
        title: '[Feature] human-observes-needs-fixes',
        user_job:
          'When designer fails a delivered story’s desk-check, human needs to see it move back to IN-PROGRESS so dev knows it’s in rework.',
        observation_falsifier:
          'reject if board does not show story back in IN-PROGRESS column after fail_desk_check round-trip'
      })
    })
    expect(createResp.status).toBe(200)

    const startResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/start_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        pickup_paraphrase: 'Picking up to verify the fail-desk-check rework return.',
        falsifier_reframing:
          'reject if start_story does not move the story into IN-PROGRESS visible to the human'
      })
    })
    expect(startResp.status).toBe(200)

    const deliverResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/deliver_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        verified_observation:
          'Drove deliver via /api/test/deliver_story; verifying DELIVERED render before fail_desk_check.'
      })
    })
    expect(deliverResp.status).toBe(200)

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const storyDelivered = win.locator(
      '#board-group-delivered-content [data-test="board-story-1"]'
    )
    await expect(storyDelivered).toContainText('human-observes-needs-fixes')

    const failResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/fail_desk_check`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        reason: 'designer flagged accessibility regression on the touched surface'
      })
    })
    expect(failResp.status).toBe(200)

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const storyInProgress = win.locator(
      '#board-group-in-progress-content [data-test="board-story-1"]'
    )
    await expect(storyInProgress).toContainText('human-observes-needs-fixes')

    for (const otherState of ['ready', 'delivered', 'icebox', 'accepted']) {
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
