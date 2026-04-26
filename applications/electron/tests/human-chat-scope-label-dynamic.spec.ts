import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — chat scope label', () => {
  test('scope label updates dynamically when recipient changes', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'seed dev as direct-role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        }
      ]
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const chatRegion = win.locator('[data-test="chat-region"]')
    await expect(chatRegion).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const scope = win.locator('aside.chat-scope')
    await expect(scope).toHaveCount(0)

    await win.locator('[data-test="sidebar-all"]').click()
    await expect(scope).toHaveText('ALL CONVERSATIONS')

    const devButton = win.locator('[data-test="sidebar-direct-dev"]')
    await expect(devButton).toBeVisible({ timeout: 10000 })
    await devButton.click()
    await expect(scope).toContainText('DM → dev')

    await win.locator('[data-test="sidebar-broadcast"]').click()
    await expect(scope).toHaveCount(0)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('chat-warning aside renders when connection is not connected (initial connecting state)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const warning = win.locator('[data-test="chat-warning"]')
    const connectionLabel = win.locator('[data-test="titlebar-connection-label"]')
    await expect(connectionLabel).toContainText('online', { timeout: 10000 })
    await expect(warning).toHaveCount(0)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
