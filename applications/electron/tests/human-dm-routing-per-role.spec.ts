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

interface FetchedMessage {
  text: string
  to?: string[] | null
}

const fetchAs = async (
  port: number,
  peer: string
): Promise<FetchedMessage[]> => {
  const resp = await fetch(`http://127.0.0.1:${port}/api/messages?as=${peer}&limit=200`)
  if (!resp.ok) throw new Error(`fetch as ${peer}: ${resp.status}`)
  return (await resp.json()) as FetchedMessage[]
}

const dmCases = [
  { target: 'pm', marker: 'DM_PM_MARKER_BPF22', other: 'dev' },
  { target: 'designer', marker: 'DM_DESIGNER_MARKER_BPF22', other: 'pm' },
  { target: 'dev', marker: 'DM_DEV_MARKER_BPF22', other: 'designer' }
] as const

test.describe('human-side E2E #476 — channel routing matrix — DMs per role', () => {
  for (const { target, marker, other } of dmCases) {
    test(`DM scoped to ${target} is visible to ${target} but NOT to ${other}`, async () => {
      const { app, win, mockGh, backendPort } = await launchWithMockGh({
        seedMessages: SEED_DIRECT_ROLES
      })
      try {
        const consoleErrors: string[] = []
        const pageErrors: Error[] = []
        win.on('console', (m) => {
          if (m.type() === 'error') consoleErrors.push(m.text())
        })
        win.on('pageerror', (e) => pageErrors.push(e))

        const sidebar = win.locator('[data-test="sidebar"]')
        await expect(sidebar).toBeVisible()
        await waitForMessagesLoaded(win, 'pm broadcast')

        const directButton = win.locator(`[data-test="sidebar-direct-${target}"]`)
        await expect(directButton).toBeVisible({ timeout: 20000 })
        await directButton.click()

        consoleErrors.length = 0
        pageErrors.length = 0

        const textarea = win.locator('[data-test="chat-input-textarea"]')
        await textarea.fill(marker)
        await textarea.press('Enter')
        await expect(textarea).toHaveValue('')

        const list = win.locator('[data-test="message-list"]')
        await expect(list).toContainText(marker, { timeout: 10000 })

        const targetMessages = await fetchAs(backendPort, target)
        const otherMessages = await fetchAs(backendPort, other)

        expect(targetMessages.some((m) => m.text === marker)).toBe(true)
        expect(otherMessages.some((m) => m.text === marker)).toBe(false)

        expect(consoleErrors).toEqual([])
        expect(pageErrors).toEqual([])
      } finally {
        await app.close()
        await mockGh.close()
      }
    })
  }
})
