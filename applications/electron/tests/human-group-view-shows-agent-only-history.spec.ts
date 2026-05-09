import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

// #463: when Ryan-as-viewer selects a group view (e.g., dev+designer),
// agent-only conversations within that group must be visible — Ryan
// observes team coordination without having been a participant. The
// pre-#463 strict-equality scope-filter hid those conversations
// because their channelId omitted Ryan ("designer+dev" vs the view's
// "designer+dev+ryan"). Diagnose-class: HIDE (data in store; filter
// over-blocks).

const SEED_GROUP_HISTORY = [
  {
    from: 'pm:claude-code',
    text: 'pm broadcast to seed direct-role',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'dev broadcast to seed direct-role',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'designer broadcast to seed direct-role',
    ts: '2026-04-22T16:02:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'AGENT_ONLY_GROUP_HISTORY_BPF463',
    ts: '2026-04-22T16:03:00Z',
    to: ['designer'],
    message_type: 'chat'
  }
]

test.describe('human-side E2E #463 — group view shows agent-only history', () => {
  test('agent-only group conversation visible when owner selects the group view', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: SEED_GROUP_HISTORY,
      seedRoster: ['pm:claude-code', 'dev:claude-code', 'designer:claude-code']
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="sidebar"]')).toBeVisible()
      await waitForMessagesLoaded(win, 'designer broadcast to seed direct-role')

      consoleErrors.length = 0
      pageErrors.length = 0

      const groupButton = win.locator('[data-test^="sidebar-group-"]').first()
      await expect(groupButton).toBeVisible({ timeout: 20000 })
      await groupButton.click()

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toContainText('AGENT_ONLY_GROUP_HISTORY_BPF463', {
        timeout: 10000
      })

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
