import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #332 rework (post-#385): clicking a sidebar role (designer)
// shifts chat scope — primary emphasis on DMs with that role,
// dim (not hide) other messages. The "DM → designer" banner
// appears. Typing + send routes with addressee=designer.
//
// Reshape from the original hide-on-select WIP per pm's 00:43
// UTC comment: #385 established nothing-hidden as the load-
// bearing semantic; #332 now rides that substrate rather than
// violating it.

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Fixture: broadcast + DM-with-designer (both directions) + noise
// DM-with-pm. The designer-scope view should emphasize the DM-with-
// designer pair and dim the broadcast + pm-DM — all four remain
// rendered.
const MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'broadcast hello',
    ts: '2026-04-22T16:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ryan:electron',
    text: 'note to designer',
    ts: '2026-04-22T16:01:00Z',
    to: ['designer'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'designer:claude-code',
    text: 'reply from designer',
    ts: '2026-04-22T16:02:00Z',
    to: ['ryan'],
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'pm note to ryan',
    ts: '2026-04-22T16:03:00Z',
    to: ['ryan'],
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-332-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-332-'))
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

test.describe('desk-check #332 rework: sidebar DM scope (post-#385 semantics)', () => {
  test('clicking designer shows banner + emphasizes DM + dims others (not hides)', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Before click: broadcast view — all 4 messages render; no banner.
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(4, {
      timeout: 5_000
    })
    await expect(win.locator('.chat-scope')).toHaveCount(0)

    // Click the designer sidebar row.
    await win.locator('[data-test="sidebar-direct-designer"]').click()

    // Banner appears + names the recipient.
    await expect(win.locator('.chat-scope')).toBeVisible()
    await expect(win.locator('.chat-scope')).toContainText('designer')

    // #385 post-condition: all 4 messages STILL render. Nothing is
    // hidden; the shift is in emphasis (dim vs primary), not in
    // visibility. This is the regression-gate for the hide-on-
    // select anti-pattern the original #332 WIP introduced.
    await expect(win.locator('[data-test="message-row"]')).toHaveCount(4, {
      timeout: 3_000
    })

    // DM-with-designer messages are primary (no dim scope).
    const designerNote = win.locator('[data-test="message-row"]', {
      hasText: 'note to designer'
    })
    const designerReply = win.locator('[data-test="message-row"]', {
      hasText: 'reply from designer'
    })
    await expect(designerNote).not.toHaveAttribute('data-scope', 'dim')
    await expect(designerReply).not.toHaveAttribute('data-scope', 'dim')

    // Broadcast message dims in the designer scope — out of the
    // current channel. pm→ryan stays primary because the "to-me"
    // rule wins unconditionally (#385 semantic: messages addressed
    // to the viewer are always primary regardless of scope). That
    // nuance matters for the rework: we don't blanket-dim
    // everything-not-in-channel; addressed-to-me is a distinct
    // always-primary tier.
    const broadcast = win.locator('[data-test="message-row"]', {
      hasText: 'broadcast hello'
    })
    const pmNote = win.locator('[data-test="message-row"]', {
      hasText: 'pm note to ryan'
    })
    await expect(broadcast).toHaveAttribute('data-scope', 'dim')
    await expect(pmNote).not.toHaveAttribute('data-scope', 'dim')

    await win.screenshot({
      path: join(EVIDENCE_DIR, '101-dm-scope-designer-dim-not-hide.png')
    })
    await app.close()
  })

  test('broadcast view: banner hidden; dim landscape inverts vs designer scope', async () => {
    // Round-trip the scope: click designer, click broadcast, verify
    // the banner is gone and the dim landscape inverts — messages
    // that were dim in designer scope now land primary, and the
    // me→designer outgoing now dims in broadcast (not addressed
    // to viewer, not broadcast).
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await win.locator('[data-test="sidebar-direct-designer"]').click()
    await expect(win.locator('.chat-scope')).toBeVisible()

    await win.locator('[data-test="sidebar-broadcast"]').click()

    // Banner gone.
    await expect(win.locator('.chat-scope')).toHaveCount(0)

    // Broadcast message back to primary (was dim in designer scope).
    const broadcast = win.locator('[data-test="message-row"]', {
      hasText: 'broadcast hello'
    })
    await expect(broadcast).not.toHaveAttribute('data-scope', 'dim')

    // Outgoing-to-designer now dims (not addressed to viewer + not
    // broadcast; the "to-me" rule can't save it here).
    const toDesigner = win.locator('[data-test="message-row"]', {
      hasText: 'note to designer'
    })
    await expect(toDesigner).toHaveAttribute('data-scope', 'dim')

    await app.close()
  })
})
