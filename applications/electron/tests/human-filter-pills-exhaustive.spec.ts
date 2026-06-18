import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

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

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(win.locator('[data-test="filter-pill-chat"]')).toHaveAttribute(
        'data-state',
        'on'
      )
      await expect(win.locator('[data-test="filter-pill-events"]')).toHaveAttribute(
        'data-state',
        'off'
      )
      await expect(win.locator('[data-test="filter-pill-bpmux"]')).toHaveAttribute(
        'data-state',
        'off'
      )

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

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      const eventsPill = win.locator('[data-test="filter-pill-events"]')
      await expect(eventsPill).toHaveAttribute('data-state', 'off')

      await eventsPill.click()
      await expect(eventsPill).toHaveAttribute('data-state', 'on')
      await expect(list).toContainText('EVENTS_CLASS_MARKER_BPF21')

      await eventsPill.click()
      await expect(eventsPill).toHaveAttribute('data-state', 'off')
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

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      const chatPill = win.locator('[data-test="filter-pill-chat"]')
      await chatPill.click()
      await expect(chatPill).toHaveAttribute('data-state', 'off')

      await expect(win.locator('[data-test="filter-pills-empty-hint"]')).toBeVisible()
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

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toBeVisible()
      await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.locator('[data-test="filter-pill-events"]').click()
      await win.locator('[data-test="filter-pill-bpmux"]').click()

      for (const cls of ['chat', 'events', 'bpmux']) {
        await expect(win.locator(`[data-test="filter-pill-${cls}"]`)).toHaveAttribute(
          'data-state',
          'on'
        )
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
