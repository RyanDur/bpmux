import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #393 team-shape audit (AC1): role set derives from runtime
// message stream, not hardcoded constants. Adding a new role
// (e.g. `qa`) requires zero renderer code changes — if the role
// appears in messages, it becomes first-class in the sidebar.
//
// Regression test strategy: seed the session with an ad-hoc
// 3-role team (ryan + dev + ops; no designer, no pm) and assert
// the sidebar renders exactly the discovered team — NOT the
// pre-#393 hardcoded ['designer', 'dev', 'pm'] list.

const AD_HOC_MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'ryan says hi',
    ts: '2026-04-23T11:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'dev says hi',
    ts: '2026-04-23T11:00:10Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ops:claude-code',
    text: 'ops says hi (new role not in pre-#393 hardcoded list)',
    ts: '2026-04-23T11:00:20Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-393-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-393-'))
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
      body: JSON.stringify(AD_HOC_MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #393 team-shape audit (AC1 — role set derives from messages)', () => {
  test('sidebar renders discovered roles, not hardcoded constants', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Positive: dev (observed) + ops (observed, not in pre-#393
    // hardcoded list) both render as sidebar entries. Viewer (ryan)
    // is excluded from the direct section.
    await expect(
      win.locator('[data-test="sidebar-direct-dev"]')
    ).toBeVisible({ timeout: 5_000 })
    await expect(
      win.locator('[data-test="sidebar-direct-ops"]')
    ).toBeVisible({ timeout: 5_000 })

    // Negative: designer + pm weren't in the observed stream, so
    // they must NOT render. Pre-#393 would have shown them from
    // the hardcoded DIRECT_ROLES list.
    await expect(win.locator('[data-test="sidebar-direct-designer"]')).toHaveCount(0)
    await expect(win.locator('[data-test="sidebar-direct-pm"]')).toHaveCount(0)

    await app.close()
  })

  test('viewer is excluded from direct section (no self-DM entry)', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // RENDERER_IDENTITY is "ryan:electron" per the useWebSocket
    // constant; shortName → "ryan". ryan shouldn't appear in the
    // direct-DM list even though their messages are in the stream.
    await expect(win.locator('[data-test="sidebar-direct-ryan"]')).toHaveCount(0, {
      timeout: 3_000
    })

    await app.close()
  })
})
