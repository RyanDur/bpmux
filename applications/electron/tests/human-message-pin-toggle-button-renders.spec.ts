import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — message pin toggle', () => {
  test('seeded chat message renders a pin-toggle button on hover affordance', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'pinnable test message',
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

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toBeVisible()
    await expect(list).toContainText('pinnable test message', { timeout: 10000 })

    consoleErrors.length = 0
    pageErrors.length = 0

    const pinButton = win.locator('[data-test="message-pin-toggle"]').first()
    await expect(pinButton).toHaveCount(1)
    await expect(pinButton).toHaveAttribute('aria-label', 'Pin message')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
