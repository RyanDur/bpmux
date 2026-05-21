import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #380: inline @mention pills in message body text. Render with
// role-color + tint + bold when the @-token matches a runtime-
// derived team role (per #393 AC1, no hardcoded team composition).
// Click or Enter/Space on a pill shifts chat scope to that role's
// DM — parity with sidebar click (#332).

const MESSAGES = [
  {
    from: 'pm:claude-code',
    text: 'ping @dev on #333 follow-up',
    ts: '2026-04-23T11:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'pm ack — @pm thanks. also @nobody-role should stay plain',
    ts: '2026-04-23T11:00:10Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-380-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-380-'))
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

test.describe('desk-check #380 inline @mention pills', () => {
  test('known-role @mention renders as .message-mention pill with data-role', async () => {
    // Falsifier (a): `@pm` in body stays plain text — regression
    // gate. pm + dev appear in the message stream so
    // `deriveTeamRoles` includes them; both @-tokens must split
    // into pills.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const pmMention = win.locator('[data-test="message-mention"][data-role="pm"]').first()
    const devMention = win.locator('[data-test="message-mention"][data-role="dev"]').first()
    await expect(pmMention).toBeVisible({ timeout: 5_000 })
    await expect(devMention).toBeVisible()
    await expect(pmMention).toHaveText('@pm')
    await expect(devMention).toHaveText('@dev')

    await app.close()
  })

  test('unknown @-token stays plain text (no .message-mention span)', async () => {
    // Falsifier (b): a role outside the derived team set renders
    // as plain text. `@nobody-role` is in the fixture but no agent
    // with that role exists — it should NOT produce a pill.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await expect(win.locator('[data-test="message-mention"][data-role="nobody-role"]')).toHaveCount(
      0
    )

    // Sanity: the @nobody-role string is still in the rendered
    // text somewhere (plain, not split).
    await expect(
      win.locator('[data-test="message-row"]', { hasText: '@nobody-role should stay plain' })
    ).toBeVisible()

    await app.close()
  })

  test('clicking a mention pill shifts chat scope to DM with that role', async () => {
    // Falsifier (c): clicking the pill doesn't scope chat. The
    // onMention handler routes through MessageList → Chat →
    // setRecipient, same path as sidebar click (#332). After the
    // click, the chat-scope banner ("DM → dev") must appear.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const devMention = win.locator('[data-test="message-mention"][data-role="dev"]').first()
    await devMention.click()

    const scopeBanner = win.locator('.chat-scope')
    await expect(scopeBanner).toBeVisible({ timeout: 3_000 })
    await expect(scopeBanner).toContainText('dev')

    await app.close()
  })
})
