import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

const SEED_MESSAGES = [
  {
    from: 'ryan:human',
    text: 'SEED_CHAT_MARKER_BPF13_FIRST',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'SEED_CHAT_MARKER_BPF13_SECOND',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  }
]

test.describe('human-side E2E #476 — read-class — message list', () => {
  test('seeded chat messages render with from + text in the message list', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: SEED_MESSAGES
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

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(list).toContainText('SEED_CHAT_MARKER_BPF13_FIRST', { timeout: 10000 })
      await expect(list).toContainText('SEED_CHAT_MARKER_BPF13_SECOND')

      const rows = win.locator('[data-test="message-row"]')
      expect(await rows.count()).toBeGreaterThanOrEqual(2)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
