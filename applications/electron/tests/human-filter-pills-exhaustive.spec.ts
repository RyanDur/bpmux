import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

const SEED_MIXED_CLASS_MESSAGES = [
  {
    from: 'pm:claude-code',
    text: 'CHAT_CLASS_MARKER_BPF21',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'INSIGHT_CLASS_MARKER_BPF21',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'insight'
  },
  {
    from: 'bpmux:activity',
    text: 'ACTIVITY_CLASS_MARKER_BPF21',
    ts: '2026-04-22T16:02:00Z',
    message_type: 'activity'
  }
]

test.describe('human-side E2E #476 — direct-action — filter pills exhaustive', () => {
  test('default-visible classes (chat + insight) show; activity hidden', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })

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

    await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21', { timeout: 10000 })
    await expect(list).toContainText('INSIGHT_CLASS_MARKER_BPF21')
    await expect(list).not.toContainText('ACTIVITY_CLASS_MARKER_BPF21')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('filter-pill-activity toggle on shows activity messages; off hides them', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })

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

    const activityPill = win.locator('[data-test="filter-pill-activity"]')
    await expect(activityPill).toHaveAttribute('aria-pressed', 'false')

    await activityPill.click()
    await expect(activityPill).toHaveAttribute('aria-pressed', 'true')
    await expect(list).toContainText('ACTIVITY_CLASS_MARKER_BPF21')

    await activityPill.click()
    await expect(activityPill).toHaveAttribute('aria-pressed', 'false')
    await expect(list).not.toContainText('ACTIVITY_CLASS_MARKER_BPF21')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('filter-pill-all reset enables all four classes simultaneously', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })

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

    const allPill = win.locator('[data-test="filter-pill-all"]')
    await expect(allPill).toHaveAttribute('aria-pressed', 'false')

    await allPill.click()
    await expect(allPill).toHaveAttribute('aria-pressed', 'true')
    for (const cls of ['chat', 'insight', 'activity', 'system']) {
      await expect(win.locator(`[data-test="filter-pill-${cls}"]`)).toHaveAttribute(
        'aria-pressed',
        'true'
      )
    }
    await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21')
    await expect(list).toContainText('INSIGHT_CLASS_MARKER_BPF21')
    await expect(list).toContainText('ACTIVITY_CLASS_MARKER_BPF21')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('multi-filter combination: insight OFF + activity ON shows chat + activity, hides insight', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: SEED_MIXED_CLASS_MESSAGES
    })

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

    await win.locator('[data-test="filter-pill-insight"]').click()
    await win.locator('[data-test="filter-pill-activity"]').click()

    await expect(list).toContainText('CHAT_CLASS_MARKER_BPF21')
    await expect(list).not.toContainText('INSIGHT_CLASS_MARKER_BPF21')
    await expect(list).toContainText('ACTIVITY_CLASS_MARKER_BPF21')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
