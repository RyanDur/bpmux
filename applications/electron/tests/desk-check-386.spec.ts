import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #386 (supersedes #346) + Ryan 2026-04-23 11:47 UTC: sidebar
// GROUPS lists all 2+ role-subsets of the combinable team — the
// observed team minus the owner (viewer) and system senders.
// Ryan: "my name should not be part of the list of combinations,
// nor should bpmux. we are constants." Detection is structural
// (viewer + messageType==='system'), not literal-name, to
// preserve #393 AC1 team-shape-independence.
//
// Fixture seeds a 5-sender stream: ryan (viewer), bpmux (system
// message_type), dev, pm, designer. Combinable set = {dev, pm,
// designer}. Groups (2+ subsets) = C(3,2) + C(3,3) = 4:
// {dev,pm}, {designer,dev}, {designer,pm}, {designer,dev,pm}.

const AD_HOC_MESSAGES = [
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
  },
  {
    from: 'designer:claude-code',
    text: 'designer here',
    ts: '2026-04-23T11:00:30Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-386-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-386-'))
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

test.describe('desk-check #386 combinatorial groups in sidebar', () => {
  test('combinable 3-role team yields exactly 4 combinatorial groups', async () => {
    // Falsifier (a): enumeration is over the combinable set (team
    // minus viewer minus system senders), not hardcoded team size.
    // Combinable = {dev, pm, designer}. Groups = C(3,2) + C(3,3)
    // = 4: {dev+pm}, {designer+dev}, {designer+pm},
    // {designer+dev+pm}.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const groupEntries = win.locator('[data-test^="sidebar-group-"]')
    await expect(groupEntries).toHaveCount(4, { timeout: 5_000 })

    const texts = await groupEntries.evaluateAll((els) =>
      els.map((e) => (e as HTMLElement).textContent?.trim() ?? '')
    )
    const sortedTexts = [...texts].sort()
    expect(sortedTexts).toEqual([
      'designer + dev',
      'designer + dev + pm',
      'designer + pm',
      'dev + pm'
    ])

    await app.close()
  })

  test('owner (ryan) and system (bpmux) are excluded from every group', async () => {
    // Falsifier (e) from Ryan 11:47 UTC: "my name should not be
    // part of the list of combinations, nor should bpmux. we are
    // constants." No group entry should name ryan or bpmux.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const groupEntries = win.locator('[data-test^="sidebar-group-"]')
    const groupKeys = await groupEntries.evaluateAll((els) =>
      els.map((e) => e.getAttribute('data-test') ?? '')
    )
    for (const key of groupKeys) {
      expect(key).not.toMatch(/ryan/)
      expect(key).not.toMatch(/bpmux/)
    }

    await app.close()
  })

  test('clicking a group sets recipient + shifts chat scope', async () => {
    // Falsifier (c): clicking doesn't set recipient. AC4 compliance
    // — same click-to-scope contract as sidebar DM entries (#332).
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const devPmGroup = win.locator('[data-test="sidebar-group-dev+pm"]')
    await devPmGroup.click()

    const scopeBanner = win.locator('.chat-scope')
    await expect(scopeBanner).toBeVisible({ timeout: 3_000 })
    // Label reads "GROUP → dev + pm" per Chat.tsx scope header.
    await expect(scopeBanner).toContainText(/dev.*pm/)

    await app.close()
  })
})
