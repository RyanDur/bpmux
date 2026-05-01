import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — message @mention pill', () => {
  test('seeded chat message with @<known-role> renders the role as a clickable pill', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'first dev-authored message — establishes dev as known role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'pm:claude-code',
          text: 'hey @dev — please look at this',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'chat'
        }
      ]
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()
      await expect(list).toContainText('please look at this', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      const mention = win.locator('[data-test="message-mention"]')
      await expect(mention.first()).toBeVisible()
      await expect(mention.first()).toContainText('@dev')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
