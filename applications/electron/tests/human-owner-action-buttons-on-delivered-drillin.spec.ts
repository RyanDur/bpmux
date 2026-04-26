import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — owner-specific — accept/reject buttons on delivered drill-in', () => {
  test('owner sees accept + reject buttons in drill-in actions for a delivered story', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-owner-actions-auth-')),
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
        title: '[Feature] owner-actions-target',
        user_job:
          'When the owner opens a delivered story drill-in, accept + reject buttons should be visible.',
        observation_falsifier:
          'reject if owner-action buttons are absent on a delivered drill-in'
      })
    }).then((r) => expect(r.status).toBe(200))

    await fetch(`http://127.0.0.1:${backendPort}/api/test/start_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        pickup_paraphrase: 'Picking up to drive deliver before owner-walk.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
    }).then((r) => expect(r.status).toBe(200))

    await fetch(`http://127.0.0.1:${backendPort}/api/test/deliver_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        verified_observation: 'Drove deliver to land in DELIVERED for the owner-action assertion.'
      })
    }).then((r) => expect(r.status).toBe(200))

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const storyRow = win.locator(
      '#board-group-delivered-content [data-test="board-story-1"]'
    )
    await expect(storyRow).toContainText('owner-actions-target')

    consoleErrors.length = 0
    pageErrors.length = 0

    await storyRow.locator('button').first().click()

    const drillIn = win.locator('[data-test="board-drill-in"]')
    await expect(drillIn).toBeVisible()

    const actions = win.locator('[data-test="drill-in-actions"]')
    await expect(actions).toBeVisible()
    await expect(win.locator('[data-test="drill-in-accept"]')).toBeVisible()
    await expect(win.locator('[data-test="drill-in-reject"]')).toBeVisible()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
