import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const SEED_MIXED_CLASS_MESSAGES = [
  {
    from: 'pm:claude-code',
    text: 'CHAT_CLASS_MARKER_BPF21',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'EVENTS_CLASS_MARKER_BPF21',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'activity'
  }
]

test.describe('human-side E2E #476 — direct-action — filter pills exhaustive (#1456 3-class model)', () => {
  test('default-visible class (chat) shows; events + bpmux start muted', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chat = new ChatPage(win)
      const list = chat.messageLog()
      await expect(list).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(chat.filterPill('chat')).toBeChecked()
      await expect(chat.filterPill('events')).not.toBeChecked()
      await expect(chat.filterPill('bpmux')).not.toBeChecked()

      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })
      await expect(list).not.toContainText('EVENTS_CLASS_MARKER_BPF21')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('filter-pill-events toggle on shows events messages; off hides them', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chat = new ChatPage(win)
      const list = chat.messageLog()
      await expect(list).toBeVisible()
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(chat.filterPill('events')).not.toBeChecked()

      await chat.toggleFilter('events')
      await expect(chat.filterPill('events')).toBeChecked()
      await expect(list).toContainText('EVENTS_CLASS_MARKER_BPF21')

      await chat.toggleFilter('events')
      await expect(chat.filterPill('events')).not.toBeChecked()
      await expect(list).not.toContainText('EVENTS_CLASS_MARKER_BPF21')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('muting the last visible pill (chat) empties the list + shows the muted hint', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chat = new ChatPage(win)
      const list = chat.messageLog()
      await expect(list).toBeVisible()
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      await chat.toggleFilter('chat')
      await expect(chat.filterPill('chat')).not.toBeChecked()

      await expect(chat.filterEmptyHint()).toBeVisible()
      await expect(list).not.toContainText('CHAT_CLASS_MARKER_BPF21')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('multi-class: enabling events + bpmux keeps chat + reveals events; all three pills on', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chat = new ChatPage(win)
      const list = chat.messageLog()
      await expect(list).toBeVisible()
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      await chat.toggleFilter('events')
      await chat.toggleFilter('bpmux')

      for (const cls of ['chat', 'events', 'bpmux']) {
        await expect(chat.filterPill(cls)).toBeChecked()
      }
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21')
      await expect(list).toContainText('EVENTS_CLASS_MARKER_BPF21')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
