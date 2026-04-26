import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow, waitForMessagesLoaded } from './helpers/launch'

const SEED_MULTI_ROLE_MESSAGES = [
  {
    from: 'pm:claude-code',
    text: 'pm broadcast to derive direct-role',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'dev broadcast to derive direct-role',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'designer broadcast to derive direct-role',
    ts: '2026-04-22T16:02:00Z',
    message_type: 'chat'
  },
  {
    from: 'ryan:human',
    text: 'pre-existing group message',
    ts: '2026-04-22T16:03:00Z',
    to: ['dev', 'designer'],
    message_type: 'chat'
  }
]

test.describe('human-side E2E #476 — channel routing matrix', () => {
  test('group message sent via sidebar-group button is visible to listed roles only', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: SEED_MULTI_ROLE_MESSAGES
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const sidebar = win.locator('[data-test="sidebar"]')
    await expect(sidebar).toBeVisible()
    await waitForMessagesLoaded(win, 'designer broadcast to derive direct-role')

    consoleErrors.length = 0
    pageErrors.length = 0

    const groupButton = win.locator('[data-test^="sidebar-group-"]').first()
    await expect(groupButton).toBeVisible({ timeout: 10000 })
    await groupButton.click()

    const textarea = win.locator('[data-test="chat-input-textarea"]')
    const groupMarker = 'GROUP_ROUTING_MARKER_BPF20'
    await textarea.fill(groupMarker)
    await textarea.press('Enter')
    await expect(textarea).toHaveValue('')

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toContainText(groupMarker, { timeout: 10000 })

    const fetchAs = async (peer: string): Promise<Array<{ text: string; to?: string[] | null }>> => {
      const resp = await fetch(
        `http://127.0.0.1:${backendPort}/api/messages?as=${peer}&limit=200`
      )
      expect(resp.status).toBe(200)
      return (await resp.json()) as Array<{ text: string; to?: string[] | null }>
    }

    const devMessages = await fetchAs('dev')
    const designerMessages = await fetchAs('designer')
    const pmMessages = await fetchAs('pm')

    const devSeen = devMessages.find((m) => m.text === groupMarker)
    expect(devSeen?.to).toBeTruthy()

    const designerSeen = designerMessages.find((m) => m.text === groupMarker)
    expect(designerSeen?.to).toBeTruthy()

    expect(pmMessages.some((m) => m.text === groupMarker)).toBe(false)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('insight filter toggle off hides insight messages; toggle on shows them again', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'a normal chat message',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'dev:claude-code',
          text: 'INSIGHT_FILTER_MARKER_BPF20',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'insight'
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
    await expect(list).toContainText('INSIGHT_FILTER_MARKER_BPF20', { timeout: 10000 })

    consoleErrors.length = 0
    pageErrors.length = 0

    const insightPill = win.locator('[data-test="filter-pill-insight"]')
    await expect(insightPill).toHaveAttribute('aria-pressed', 'true')

    await insightPill.click()
    await expect(insightPill).toHaveAttribute('aria-pressed', 'false')
    await expect(list).not.toContainText('INSIGHT_FILTER_MARKER_BPF20')

    await insightPill.click()
    await expect(insightPill).toHaveAttribute('aria-pressed', 'true')
    await expect(list).toContainText('INSIGHT_FILTER_MARKER_BPF20')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
