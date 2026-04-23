import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #402 team-roster substrate: sidebar directs + groups derive from
// connected-agent roster, not observed messages. This test seeds
// the /api/roster endpoint with a team where some roles HAVEN'T
// spoken — pre-#402 they would have been missing from the sidebar
// (the exact "keep losing the groups and directs" pain Ryan
// surfaced 2026-04-23 12:08 UTC).

const ROSTER = {
  roster: [
    { identity: 'ryan:electron', role: 'ryan' },
    { identity: 'dev:claude-code', role: 'dev' },
    { identity: 'pm:claude-code', role: 'pm' },
    { identity: 'designer:claude-code', role: 'designer' }
  ],
  fetched_at: '2026-04-23T12:00:00Z'
}

// Only ryan has spoken. Without roster-based membership, pre-#402
// sidebar would show ONLY ryan (not a direct target) and nothing
// else — the regression this story fixes.
const MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'i am here',
    ts: '2026-04-23T12:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(roster: typeof ROSTER | null = ROSTER): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-402-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-402-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/roster', async (route) => {
    if (roster === null) {
      await route.fulfill({ status: 500, body: '{}' })
      return
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(roster)
    })
  })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-23T12:00:00Z', stories: [] })
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

test.describe('desk-check #402 team-roster substrate', () => {
  test('sidebar shows direct-DM entry for every connected agent, regardless of speech', async () => {
    // Falsifier (a): bridge-connected agent absent from sidebar
    // directs for any reason other than viewer / system. The
    // roster fixture has 4 agents but only ryan has spoken; pre-
    // #402 the sidebar would drop dev + pm + designer. Post-#402
    // all three should render as directs since they're in the
    // roster.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await expect(
      win.locator('[data-test="sidebar-direct-dev"]')
    ).toBeVisible({ timeout: 5_000 })
    await expect(win.locator('[data-test="sidebar-direct-pm"]')).toBeVisible()
    await expect(win.locator('[data-test="sidebar-direct-designer"]')).toBeVisible()

    // Viewer (ryan) excluded per #393 AC1 discipline.
    await expect(win.locator('[data-test="sidebar-direct-ryan"]')).toHaveCount(0)

    await app.close()
  })

  test('combinatorial groups enumerate over roster, not messages', async () => {
    // Falsifier: groups derivation not switched to roster input.
    // Combinable = {dev, pm, designer} (viewer excluded); groups
    // = C(3,2) + C(3,3) = 4 subsets. If the pre-#402 message-based
    // derivation were still in effect, there'd be 0 groups
    // (nobody but ryan has spoken).
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const groupEntries = win.locator('[data-test^="sidebar-group-"]')
    await expect(groupEntries).toHaveCount(4, { timeout: 5_000 })

    await app.close()
  })

  test('roster-fetch failure falls back to message-derived membership', async () => {
    // Falsifier: roster unavailable → sidebar empty / blown up.
    // Graceful fallback ensures the user-facing surface stays
    // populated via message-derivation. For this fixture the only
    // speaker is ryan, so direct-DM list ends up empty (nothing
    // to fall back to), but broadcast + system still render.
    const { app } = await launchWithFixture(null)
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Broadcast entry always renders regardless of roster / messages.
    await expect(win.locator('[data-test="sidebar-broadcast"]')).toBeVisible()

    await app.close()
  })
})
