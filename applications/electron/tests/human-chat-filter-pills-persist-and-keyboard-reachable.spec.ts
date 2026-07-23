import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MIXED_CLASS_MESSAGES = [
  {
    id: 'm1',
    from: 'designer:claude-code',
    text: 'Plain chat message — no gutter glyph.',
    ts: '2026-04-22T15:00:00Z',
    message_type: 'chat'
  },
  {
    id: 'm2',
    from: 'dev:claude-code',
    text: 'This is an insight — ◊ glyph prefix expected.',
    ts: '2026-04-22T15:01:00Z',
    message_type: 'insight'
  },
  {
    id: 'm3',
    from: 'pm:claude-code',
    text: 'This is an activity message — • glyph expected.',
    ts: '2026-04-22T15:02:00Z',
    message_type: 'activity'
  },
  {
    id: 'm4',
    from: 'owner:human',
    text: 'This is a system message — ⚙ glyph expected.',
    ts: '2026-04-22T15:03:00Z',
    message_type: 'system'
  },
  {
    id: 'm5',
    from: 'designer:claude-code',
    text: 'Back to chat — no glyph.',
    ts: '2026-04-22T15:04:00Z',
    message_type: 'chat'
  },
  {
    id: 'm6',
    from: 'dev:claude-code',
    text: 'Another insight.',
    ts: '2026-04-22T15:05:00Z',
    message_type: 'insight'
  }
]

const launchWithFixture = async (
  userDataDir?: string
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
  udd: string
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-'))
  const udd = userDataDir ?? mkdtempSync(join(tmpdir(), 'bpmux-userdata-349-'))
  const { app } = await launchIsolatedElectron({ dataBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MIXED_CLASS_MESSAGES)
    })
  })
  await win.reload()
  return { app, udd }
}

test.describe('desk-check #349 richer filter', () => {
  test('default state: 3 pills present, chat on, events + bpmux muted', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.region()).toBeVisible()

    const pillContainer = chat.filterRegion()
    await expect(pillContainer).toBeVisible()

    await expect(chat.filterPill('chat')).toBeChecked()
    await expect(chat.filterPill('events')).not.toBeChecked()
    await expect(chat.filterPill('bpmux')).not.toBeChecked()

    await pillContainer.screenshot({
      path: join(EVIDENCE_DIR, '10-filter-default-chat-only.png')
    })

    await app.close()
  })

  test('toggle bpmux ON: pill becomes active', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.region()).toBeVisible()

    await expect(chat.filterPill('bpmux')).not.toBeChecked()
    await chat.toggleFilter('bpmux')
    await expect(chat.filterPill('bpmux')).toBeChecked()

    await chat.region().screenshot({
      path: join(EVIDENCE_DIR, '11-filter-bpmux-on.png')
    })

    await app.close()
  })

  test('persistence: filter survives relaunch via localStorage', async () => {
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-349-persist-'))
    const first = await launchWithFixture(udd)
    const win1 = await first.app.firstWindow()
    const chat1 = new ChatPage(win1)
    await expect(chat1.region()).toBeVisible()

    await chat1.toggleFilter('bpmux')
    await expect(chat1.filterPill('bpmux')).toBeChecked()
    await first.app.close()

    const second = await launchWithFixture(udd)
    const win2 = await second.app.firstWindow()
    const chat2 = new ChatPage(win2)
    await expect(chat2.region()).toBeVisible()

    await expect(chat2.filterPill('bpmux')).toBeChecked()

    await chat2.filterRegion().screenshot({
      path: join(EVIDENCE_DIR, '12-filter-persisted-after-reopen.png')
    })

    await second.app.close()
  })

  test('keyboard: pills Tab-reachable', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.region()).toBeVisible()

    await chat.filterPill('chat').focus()
    const reached: string[] = []
    for (let i = 0; i < 3; i++) {
      const label = await win.evaluate(
        () => document.activeElement?.getAttribute('aria-label') ?? ''
      )
      reached.push(label)
      await win.keyboard.press('Tab')
    }
    expect(reached.some((l) => l.startsWith('chat messages'))).toBe(true)
    expect(reached.some((l) => l.startsWith('events messages'))).toBe(true)
    expect(reached.some((l) => l.startsWith('bpmux messages'))).toBe(true)

    await app.close()
  })
})
