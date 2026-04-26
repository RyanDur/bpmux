import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — drill-in comment input', () => {
  test('comment send button is disabled when input is empty, enabled when typed', async () => {
    const authTokenPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-476-comment-input-auth-')),
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
        title: '[Feature] comment-input-target',
        user_job:
          'When the human opens a drill-in, the comment send button should be disabled until they type something.',
        observation_falsifier:
          'reject if comment send button is enabled with empty input OR remains disabled after typing'
      })
    }).then((r) => expect(r.status).toBe(200))

    await win.reload()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()
    const storyRow = win.locator(
      '#board-group-icebox-content [data-test="board-story-1"]'
    )
    await expect(storyRow).toContainText('comment-input-target')

    await storyRow.locator('button').first().click()
    const drillIn = win.locator('[data-test="board-drill-in"]')
    await expect(drillIn).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const sendButton = win.locator('[data-test="drill-in-comment-send"]')
    await expect(sendButton).toBeVisible()
    await expect(sendButton).toBeDisabled()

    const commentInput = win.locator('[data-test="drill-in-comment-input"]')
    await commentInput.fill('Some non-trivial comment text.')
    await expect(sendButton).toBeEnabled()

    await commentInput.fill('')
    await expect(sendButton).toBeDisabled()

    await commentInput.fill('   ')
    await expect(sendButton).toBeDisabled()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
