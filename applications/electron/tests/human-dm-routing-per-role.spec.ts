import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'
import { SidebarPage } from './pages/sidebarPage'

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

test.describe('human-side #1595 — selecting a direct role opens its scoped CHAT (terminals are now ＋-spawned, not role-derived)', () => {
  for (const { target } of roomCases) {
    test(`selecting ${target} opens the ${target} chat, not a terminal`, async () => {
      const { app, win, mockGh } = await launchWithMockGh({
        seedMessages: SEED_DIRECT_ROLES,
        seedRoster: ['pm:claude-code', 'dev:claude-code', 'designer:claude-code']
      })
      try {
        const chat = new ChatPage(win)
        const sidebar = new SidebarPage(win)
        await expect(sidebar.nav()).toBeVisible()
        await waitForMessagesLoaded(win, 'pm broadcast')

        const directButton = sidebar.directChannel(target)
        await expect(directButton).toBeVisible({ timeout: 20000 })
        await directButton.click()

        await expect(chat.messageLog()).toBeVisible({ timeout: 10000 })
        await expect(chat.terminalOutput()).toHaveCount(0)
      } finally {
        await app.close()
        await mockGh.close()
      }
    })
  }
})
