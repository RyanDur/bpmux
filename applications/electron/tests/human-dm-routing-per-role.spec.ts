import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

const SEED_DIRECT_ROLES = [
  {
    from: 'pm:claude-code',
    text: 'pm broadcast',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'dev broadcast',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'designer broadcast',
    ts: '2026-04-22T16:02:00Z',
    message_type: 'chat'
  }
]

const roomCases = [{ target: 'pm' }, { target: 'designer' }, { target: 'dev' }] as const

test.describe('human-side E2E #476 — room=terminal — selecting a direct role opens its terminal', () => {
  for (const { target } of roomCases) {
    test(`selecting ${target} opens the ${target} terminal-room (not a message thread)`, async () => {
      const { app, win, mockGh } = await launchWithMockGh({
        seedMessages: SEED_DIRECT_ROLES,
        seedRoster: ['pm:claude-code', 'dev:claude-code', 'designer:claude-code']
      })
      try {
        const sidebar = win.locator('[data-test="sidebar"]')
        await expect(sidebar).toBeVisible()
        await waitForMessagesLoaded(win, 'pm broadcast')

        const directButton = win.locator(`[data-test="sidebar-direct-${target}"]`)
        await expect(directButton).toBeVisible({ timeout: 20000 })
        await directButton.click()

        const terminal = win.locator(`[data-test="terminal-${target}"]`)
        await expect(terminal).toBeVisible({ timeout: 10000 })

        await expect(win.locator('[data-test="message-list"]')).toHaveCount(0)
        await expect(win.locator('[data-test="chat-input-textarea"]')).toHaveCount(0)
      } finally {
        await app.close()
        await mockGh.close()
      }
    })
  }
})
