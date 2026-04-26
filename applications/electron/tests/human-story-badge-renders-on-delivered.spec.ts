import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — story badge', () => {
  test('delivered Feature renders the awaiting-owner badge', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-badge-auth-')),
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
        title: '[Feature] badge-render-target',
        user_job:
          'When dev delivers a Feature story, human needs to see the awaiting-owner badge on the row.',
        observation_falsifier:
          'reject if delivered Feature does not render the awaiting-owner badge variant'
      })
    })
    expect(createResp.status).toBe(200)

    const startResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/start_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        pickup_paraphrase: 'Picking up to drive the badge-render assertion.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
    })
    expect(startResp.status).toBe(200)

    const deliverResp = await fetch(`http://127.0.0.1:${backendPort}/api/test/deliver_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        verified_observation: 'Drove the chain to deliver so the badge derivation can fire.'
      })
    })
    expect(deliverResp.status).toBe(200)

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const storyRow = win.locator(
      '#board-group-delivered-content [data-test="board-story-1"]'
    )
    await expect(storyRow).toContainText('badge-render-target')

    consoleErrors.length = 0
    pageErrors.length = 0

    const badge = storyRow.locator('[data-test="story-badge"]')
    await expect(badge).toBeVisible()
    await expect(badge).toHaveAttribute('data-badge-kind', 'accepted')
    await expect(badge).toHaveAttribute('data-state', 'delivered')
    await expect(badge).toContainText('awaiting owner')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
