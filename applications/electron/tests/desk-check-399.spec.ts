import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #399 follow-up to #386: `deriveDirectRoles` now filters system
// senders in addition to the viewer. Ryan 2026-04-23 11:51 UTC:
// "bpmux is not something we have conversations with. it is the
// medium in which we communicate." The direct-DM picker must not
// offer bpmux as a target — same semantic the combinatorial
// groups helper honors, now extended to the DMs section.

const MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'ryan here',
    ts: '2026-04-23T11:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'bpmux',
    text: 'system marker',
    ts: '2026-04-23T11:00:05Z',
    to: null,
    addressee: null,
    message_type: 'system'
  },
  {
    from: 'dev:claude-code',
    text: 'dev here',
    ts: '2026-04-23T11:00:10Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'pm here',
    ts: '2026-04-23T11:00:20Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-399-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-399-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-23T11:00:00Z', stories: [] })
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

test.describe('desk-check #399 deriveDirectRoles excludes system senders', () => {
  test('bpmux is NOT a direct-DM entry despite appearing in message stream', async () => {
    // Falsifier (a): bpmux appears as a sidebar-direct-* entry
    // post-fix. Pre-#399, the direct-DM list only excluded the
    // viewer; bpmux (system sender) leaked into the list. Post-
    // #399, the structural `messageType === 'system'` detection
    // filters it out.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await expect(win.locator('[data-test="sidebar-direct-bpmux"]')).toHaveCount(0)

    // Sanity: the other agents that spoke non-system messages
    // still render as first-class DM entries.
    await expect(win.locator('[data-test="sidebar-direct-dev"]')).toBeVisible()
    await expect(win.locator('[data-test="sidebar-direct-pm"]')).toBeVisible()

    await app.close()
  })

  test('viewer (ryan) also remains excluded (regression gate for prior behavior)', async () => {
    // The original `deriveDirectRoles` behavior (viewer excluded)
    // is unchanged; this test locks it in so the new system-
    // exclusion didn't accidentally break the viewer check.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await expect(win.locator('[data-test="sidebar-direct-ryan"]')).toHaveCount(0)

    await app.close()
  })
})
