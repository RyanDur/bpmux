import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — drill-in reject form', () => {
  test('reject button opens textarea + send is gated on non-empty trimmed note', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-reject-form-auth-')),
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
        title: '[Feature] reject-form-target',
        user_job: 'When the owner rejects a delivered story, the form gates send on a non-empty note.',
        observation_falsifier: 'reject if send button enables on empty/whitespace note'
      })
    }).then((r) => expect(r.status).toBe(200))

    await fetch(`http://127.0.0.1:${backendPort}/api/test/start_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        pickup_paraphrase: 'Driving the chain so reject form is reachable.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
    }).then((r) => expect(r.status).toBe(200))

    await fetch(`http://127.0.0.1:${backendPort}/api/test/deliver_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        number: 1,
        verified_observation: 'Drove deliver to land in DELIVERED so the reject button activates.'
      })
    }).then((r) => expect(r.status).toBe(200))

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const storyRow = win.locator(
      '#board-group-delivered-content [data-test="board-story-1"]'
    )
    await expect(storyRow).toContainText('reject-form-target')

    consoleErrors.length = 0
    pageErrors.length = 0

    await storyRow.locator('button').first().click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

    await win.locator('[data-test="drill-in-reject"]').click()

    const noteInput = win.locator('[data-test="drill-in-reject-note"]')
    await expect(noteInput).toBeVisible()
    const sendButton = win.locator('[data-test="drill-in-reject-send"]')
    await expect(sendButton).toBeDisabled()

    await noteInput.fill('needs rework — accessibility regression on the touched surface')
    await expect(sendButton).toBeEnabled()

    await noteInput.fill('   ')
    await expect(sendButton).toBeDisabled()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
