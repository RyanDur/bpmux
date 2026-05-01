import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const FIXTURE_MESSAGES = [
  {
    from: 'designer:claude-code',
    text: 'broadcast from designer',
    ts: '2026-04-24T02:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ryan:electron',
    text: 'dm ryan to designer',
    ts: '2026-04-24T02:01:00Z',
    to: ['designer'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'group dev to designer+pm',
    ts: '2026-04-24T02:02:00Z',
    to: ['designer', 'pm'],
    addressee: null,
    message_type: 'chat'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-420-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-420-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()

  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-24T02:00:00Z', stories: [] })
    })
  })
  await win.route('**/api/messages*', async (route, request) => {
    const url = request.url()
    if (url.includes('message_type=activity')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: '[]'
      })
      return
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(FIXTURE_MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #420 per-message channel tag', () => {
  test('channel-tags render in TUI-parity format + owner elided', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    // Default view is `broadcast` — only DMs/groups visible when
    // their channel is selected, only broadcasts visible by default.
    // Test asserts the cross-section of channel tags so flip into
    // ALL view via the sidebar's `all` selector.
    await win.locator('[data-test="sidebar-all"]').click()
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(3, {
      timeout: 5_000
    })

    const tags = await win
      .locator('[data-test="message-channel"]')
      .evaluateAll((els) => els.map((e) => (e as HTMLElement).textContent?.trim() ?? ''))

    expect(tags.length).toBeGreaterThanOrEqual(1)
    expect(tags).toContain('[broadcast]')
    expect(tags).toContain('[designer]')
    expect(tags).toContain('[designer+dev+pm]')

    for (const tag of tags) {
      expect(tag).not.toMatch(/ryan/)
    }

    await app.close()
  })
})
