import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #401 (#393 AC2): dynamic role palette retires the static
// --role-* CSS tokens. Any role — including one that never had a
// CSS entry, e.g. `ops` — renders with a distinct hashed color.
// Deterministic: same role short-name always yields the same
// color.

const ROSTER = {
  roster: [
    { identity: 'ryan:electron', role: 'ryan' },
    { identity: 'dev:claude-code', role: 'dev' },
    { identity: 'pm:claude-code', role: 'pm' },
    { identity: 'ops:claude-code', role: 'ops' }
  ],
  fetched_at: '2026-04-23T12:20:00Z'
}

const MESSAGES = [
  {
    from: 'dev:claude-code',
    text: 'hello @ops',
    ts: '2026-04-23T12:20:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ops:claude-code',
    text: 'hi @dev',
    ts: '2026-04-23T12:20:10Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-401-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-401-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/roster', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(ROSTER)
    })
  })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-23T12:20:00Z', stories: [] })
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

test.describe('desk-check #401 dynamic role palette', () => {
  test('ops role (no static CSS token) renders with distinct non-gray color', async () => {
    // Post-#401, inline style applied via roleColorFor produces a
    // saturated HSL color for ANY role, not the neutral fallback.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const opsAvatar = win
      .locator('[data-test="message-avatar"][data-role="ops"]')
      .first()
    await expect(opsAvatar).toBeVisible({ timeout: 5_000 })

    const color = await opsAvatar.evaluate((el) => getComputedStyle(el).color)
    const m = /rgb\((\d+),\s*(\d+),\s*(\d+)\)/.exec(color)
    expect(m).not.toBeNull()
    const r = Number(m![1])
    const g = Number(m![2])
    const b = Number(m![3])
    const saturated = r !== g || g !== b
    expect(saturated).toBe(true)

    await app.close()
  })

  test('same role name produces identical color across avatar + mention pill', async () => {
    // Deterministic hash: two surfaces styled from the same
    // roleColorFor call must match exactly.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const opsAvatar = win
      .locator('[data-test="message-avatar"][data-role="ops"]')
      .first()
    const opsMention = win
      .locator('[data-test="message-mention"][data-role="ops"]')
      .first()
    await expect(opsAvatar).toBeVisible()
    await expect(opsMention).toBeVisible()

    const avatarColor = await opsAvatar.evaluate(
      (el) => getComputedStyle(el).color
    )
    const mentionColor = await opsMention.evaluate(
      (el) => getComputedStyle(el).color
    )
    expect(avatarColor).toBe(mentionColor)

    await app.close()
  })
})
