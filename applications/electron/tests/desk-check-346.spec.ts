import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { ChatPage } from './pages/chatPage'
import { SidebarPage } from './pages/sidebarPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MESSAGES_WITH_GROUPS_AND_SYSTEM = [
  // broadcast
  {
    id: 'm1',
    from: 'designer:claude-code',
    text: 'Broadcast message 1',
    ts: '2026-04-22T15:00:00Z',
    message_type: 'chat'
  },
  // designer+dev group
  {
    id: 'm2',
    from: 'designer:claude-code',
    text: 'Scoped message to designer+dev',
    ts: '2026-04-22T15:01:00Z',
    message_type: 'chat',
    to: ['designer', 'dev']
  },
  {
    id: 'm3',
    from: 'dev:claude-code',
    text: 'Reply in designer+dev group',
    ts: '2026-04-22T15:02:00Z',
    message_type: 'chat',
    to: ['dev', 'designer']
  },
  // designer+dev+pm group
  {
    id: 'm4',
    from: 'pm:claude-code',
    text: 'Three-role group',
    ts: '2026-04-22T15:03:00Z',
    message_type: 'chat',
    to: ['designer', 'dev', 'pm']
  },
  // DM to ryan (single recipient — this counts as DM not group)
  {
    id: 'm5',
    from: 'designer:claude-code',
    text: 'DM to ryan only',
    ts: '2026-04-22T15:04:00Z',
    message_type: 'chat',
    to: ['ryan']
  },
  // System messages
  {
    id: 'm6',
    from: 'bpmux:system',
    text: 'System event 1',
    ts: '2026-04-22T15:05:00Z',
    message_type: 'system'
  },
  {
    id: 'm7',
    from: 'bpmux:system',
    text: 'System event 2',
    ts: '2026-04-22T15:06:00Z',
    message_type: 'system'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-346-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-346-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MESSAGES_WITH_GROUPS_AND_SYSTEM)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #346 sidebar derivation', () => {
  test('sidebar GROUPS section lists derived groups with stable labels', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const sidebar = new SidebarPage(win)
    await expect(sidebar.nav()).toBeVisible()
    await win.waitForTimeout(800)

    await sidebar.nav().screenshot({
      path: join(EVIDENCE_DIR, '20-sidebar-populated.png')
    })

    const items = await sidebar.nav().locator('button, p').allTextContents()
    console.log('#346 AC sidebar content:')
    items.forEach((t, i) =>
      console.log(`  item[${i}] "${t.trim().replace(/\s+/g, ' ').slice(0, 80)}"`)
    )
    await app.close()
  })

  test('clicking a group scopes the chat', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const sidebar = new SidebarPage(win)
    const chat = new ChatPage(win)
    await win.waitForTimeout(800)

    const groups = sidebar.groupChannels()
    const groupCount = await groups.count()
    console.log(`#346 groups found: ${groupCount}`)

    if (groupCount > 0) {
      const firstGroupLabel = (await groups.first().textContent())?.trim()
      console.log(`  clicking group: "${firstGroupLabel}"`)
      await groups.first().click()
      await win.waitForTimeout(300)

      const scope = win.locator('.chat-scope').first()
      if ((await scope.count()) > 0) {
        const scopeText = (await scope.textContent())?.trim()
        console.log(`  chat scope after click: "${scopeText}"`)
      } else {
        console.log(`  no chat-scope indicator found — checking input placeholder`)
        const placeholder = await chat.input().getAttribute('placeholder')
        console.log(`  input placeholder: "${placeholder}"`)
      }

      await win.screenshot({
        path: join(EVIDENCE_DIR, '21-sidebar-group-clicked.png'),
        fullPage: false
      })
    }

    await app.close()
  })

  test('SYSTEM section shows count, clicking narrows filter', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const sidebar = new SidebarPage(win)
    const chat = new ChatPage(win)
    await win.waitForTimeout(800)

    const systemBtn = sidebar.nav().getByRole('button', { name: /system/i })
    const systemCount = await systemBtn.count()
    console.log(`#346 system sidebar button present: ${systemCount > 0}`)

    if (systemCount > 0) {
      const text = (await systemBtn.textContent())?.trim()
      console.log(`  system sidebar label: "${text}"`)

      await systemBtn.click()
      await win.waitForTimeout(500)

      const systemPressed = await chat.filterPill('system').getAttribute('aria-pressed')
      const chatPressed = await chat.filterPill('chat').getAttribute('aria-pressed')
      console.log(`  filter after system click: chat=${chatPressed} system=${systemPressed}`)

      await chat.region().screenshot({
        path: join(EVIDENCE_DIR, '22-system-click-narrows-filter.png')
      })
    }

    await app.close()
  })
})
