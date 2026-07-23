import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'
import { SidebarPage } from './pages/sidebarPage'

const FIXTURE_MESSAGES = [
  {
    from: 'designer:claude-code',
    text: 'broadcast from designer',
    ts: '2026-04-24T02:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'owner:electron',
    text: 'dm owner to designer',
    ts: '2026-04-24T02:01:00Z',
    to: ['designer'],
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'group dev to designer+pm',
    ts: '2026-04-24T02:02:00Z',
    to: ['designer', 'pm'],
    message_type: 'chat'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-420-'))
  const { app } = await launchIsolatedElectron({
    dataBase,
    userDataDir: udd,
    seedMessages: FIXTURE_MESSAGES
  })
  const win = await app.firstWindow()
  await waitForMessagesLoaded(win, 'broadcast from designer')
  return { app }
}

test.describe('desk-check #420 per-message channel tag', () => {
  test('channel-tags render in TUI-parity format + owner elided', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    // Default view is `broadcast` — only DMs/groups visible when
    // their channel is selected, only broadcasts visible by default.
    // Test asserts the cross-section of channel tags so flip into
    // ALL view via the sidebar's `all` selector.
    await expect(chat.region()).toBeVisible()
    await new SidebarPage(win).allChannel().click()
    await expect(chat.messageRows()).toHaveCount(3, {
      timeout: 5_000
    })

    const tags = await chat
      .channelTags()
      .evaluateAll((els) => els.map((e) => (e as HTMLElement).textContent?.trim() ?? ''))

    expect(tags.length).toBeGreaterThanOrEqual(1)
    expect(tags).toContain('[broadcast]')
    expect(tags).toContain('[designer]')
    expect(tags).toContain('[designer+dev+pm]')

    for (const tag of tags) {
      expect(tag).not.toMatch(/owner/)
    }

    await app.close()
  })
})
