import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const RECENT_COUNT = 200
const OLDER_COUNT = 5

const olderMessages = Array.from({ length: OLDER_COUNT }, (_, i) => ({
  from: 'pm:claude-code',
  text: `older message ${i}`,
  ts: `2026-05-03T05:${String(i).padStart(2, '0')}:00Z`,
  message_type: 'chat'
}))

const recentMessages = Array.from({ length: RECENT_COUNT }, (_, i) => {
  const minute = String(i % 60).padStart(2, '0')
  const hour = String(10 + Math.floor(i / 60)).padStart(2, '0')
  return {
    from: 'dev:claude-code',
    text: `seed message ${i}`,
    ts: `2026-05-03T${hour}:${minute}:00Z`,
    message_type: 'chat'
  }
})

const ALL_MESSAGES = [...olderMessages, ...recentMessages]

test.describe('human-side E2E #1153 — scroll-up triggers paged-history fetch', () => {
  test('scroll near visual top fires loadOlder + loading-pill renders', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: ALL_MESSAGES
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await waitForMessagesLoaded(win, 'seed message 199')

      const ul = new ChatPage(win).messageScroller()
      await expect(ul).toBeVisible()
      await ul.evaluate((el) => {
        const target = -(el.scrollHeight - el.clientHeight)
        el.scrollTop = target
        el.dispatchEvent(new Event('scroll'))
      })

      const loadingPill = new ChatPage(win).loadingOlderPill()
      await expect(loadingPill).toBeVisible({ timeout: 2000 })

      await expect(loadingPill).toHaveCount(0, { timeout: 5000 })

      for (let i = 0; i < OLDER_COUNT; i++) {
        await expect(ul).toContainText(`older message ${i}`)
      }

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
