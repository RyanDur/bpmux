import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — drill-in body content', () => {
  test('drill-in renders the story user_job + observation_falsifier markers from the body', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-drill-body-auth-')),
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

    const userJobMarker = 'USER_JOB_DRILL_IN_BODY_BPF12'
    const falsifierMarker = 'FALSIFIER_DRILL_IN_BODY_BPF12'

    await fetch(`http://127.0.0.1:${backendPort}/api/test/create_story`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: '[Feature] drill-body-target',
        user_job: userJobMarker,
        observation_falsifier: falsifierMarker
      })
    }).then((r) => expect(r.status).toBe(200))

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()
    const storyRow = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
    await expect(storyRow).toContainText('drill-body-target')

    await storyRow.locator('button').first().click()
    const drillIn = win.locator('[data-test="board-drill-in"]')
    await expect(drillIn).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    await expect(drillIn).toContainText(userJobMarker, { timeout: 10000 })
    await expect(drillIn).toContainText(falsifierMarker)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
