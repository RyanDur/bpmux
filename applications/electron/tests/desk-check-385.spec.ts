import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #385: channel selection shifts scope + emphasis, never visibility.
// "Nothing hidden from owner" principle (Ryan 2026-04-22 ~22:48
// UTC). The observation-falsifier from the pickup-gate paraphrase:
// "I would reject if, while viewing any channel, any message
// (broadcast / DM I'm part of / group I'm part of / system)
// disappears from the chat stream."

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Fixture covers all visibility-testable shapes simultaneously. Each
// message has a distinguishing text so assertions can target by
// content. Viewer = ryan.
const MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'broadcast-from-ryan',
    ts: '2026-04-22T16:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'broadcast-from-designer',
    ts: '2026-04-22T16:01:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ryan:electron',
    text: 'dm-ryan-to-designer',
    ts: '2026-04-22T16:02:00Z',
    to: ['designer'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'dm-designer-to-ryan',
    ts: '2026-04-22T16:03:00Z',
    to: ['ryan'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'dm-pm-to-ryan',
    ts: '2026-04-22T16:04:00Z',
    to: ['ryan'],
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  // #385 rework substrate: port-isolation is filed as #394; until it
  // lands, this spec uses the same bare-launch pattern as other
  // desk-check specs. CI runs in isolated containers; live-user
  // conflict risk is documented via #394.
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-385-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-385-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-22T16:00:00Z', stories: [] })
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
      body: JSON.stringify(MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #385: channels shift scope, never hide', () => {
  test('broadcast view renders ALL messages; non-broadcast non-to-me dimmed', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // All 5 messages render regardless of their channel/type — the
    // core "nothing hidden" falsifier.
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(5, {
      timeout: 5_000
    })

    // Scope-attribute assertions (broadcast view, recipient=null):
    // - broadcast messages → primary (no dim)
    // - DMs to ryan (addressed-to-viewer) → primary (rule 1)
    // - ryan's outgoing DM to designer → dim (not to viewer, not
    //   broadcast-in-broadcast)
    const broadcastRyan = win.locator('[data-test="message-row"]', {
      hasText: 'broadcast-from-ryan'
    })
    await expect(broadcastRyan).not.toHaveAttribute('data-scope', 'dim')

    const broadcastDesigner = win.locator('[data-test="message-row"]', {
      hasText: 'broadcast-from-designer'
    })
    await expect(broadcastDesigner).not.toHaveAttribute('data-scope', 'dim')

    const dmToRyan = win.locator('[data-test="message-row"]', {
      hasText: 'dm-designer-to-ryan'
    })
    await expect(dmToRyan).not.toHaveAttribute('data-scope', 'dim')

    const dmPmToRyan = win.locator('[data-test="message-row"]', {
      hasText: 'dm-pm-to-ryan'
    })
    await expect(dmPmToRyan).not.toHaveAttribute('data-scope', 'dim')

    // Ryan's outgoing DM to designer: not addressed to viewer
    // (to=[designer], not ryan), not broadcast; dimmed.
    const dmRyanToDesigner = win.locator('[data-test="message-row"]', {
      hasText: 'dm-ryan-to-designer'
    })
    await expect(dmRyanToDesigner).toHaveAttribute('data-scope', 'dim')

    await win.screenshot({
      path: join(EVIDENCE_DIR, '103-385-broadcast-scope.png')
    })
    await app.close()
  })

  test('selecting designer in sidebar: DMs-with-designer primary, others still visible + dim', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const designerRow = win
      .locator('button', { hasText: 'designer' })
      .first()
    await designerRow.click()

    // All 5 messages still render — "nothing hidden" holds.
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(5, {
      timeout: 3_000
    })

    // DMs between ryan + designer = primary (current channel match
    // or addressed-to-viewer).
    const dmRyanToDesigner = win.locator('[data-test="message-row"]', {
      hasText: 'dm-ryan-to-designer'
    })
    await expect(dmRyanToDesigner).not.toHaveAttribute('data-scope', 'dim')

    const dmDesignerToRyan = win.locator('[data-test="message-row"]', {
      hasText: 'dm-designer-to-ryan'
    })
    await expect(dmDesignerToRyan).not.toHaveAttribute('data-scope', 'dim')

    // DM from pm to ryan: still primary (addressed-to-viewer — rule 1
    // wins regardless of current channel). DMs-to-me-always-primary
    // per designer spec.
    const dmPmToRyan = win.locator('[data-test="message-row"]', {
      hasText: 'dm-pm-to-ryan'
    })
    await expect(dmPmToRyan).not.toHaveAttribute('data-scope', 'dim')

    // Broadcast messages when viewing a DM channel: not current-
    // channel-match, not addressed-to-viewer, not broadcast-in-
    // broadcast-view → dim.
    const broadcastDesigner = win.locator('[data-test="message-row"]', {
      hasText: 'broadcast-from-designer'
    })
    await expect(broadcastDesigner).toHaveAttribute('data-scope', 'dim')

    const broadcastRyan = win.locator('[data-test="message-row"]', {
      hasText: 'broadcast-from-ryan'
    })
    await expect(broadcastRyan).toHaveAttribute('data-scope', 'dim')

    await win.screenshot({
      path: join(EVIDENCE_DIR, '104-385-dm-designer-scope.png')
    })
    await app.close()
  })
})
