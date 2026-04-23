import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-369-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-369-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-22T16:00:00Z', stories: [] })
    })
  })
  // A few messages so the message list actually renders avatars.
  const messages = [
    {
      from: 'designer:claude-code',
      text: 'first message',
      ts: '2026-04-22T16:00:00Z',
      to: null,
      addressee: null,
      message_type: 'chat'
    },
    {
      from: 'pm:claude-code',
      text: 'second message',
      ts: '2026-04-22T16:01:00Z',
      to: null,
      addressee: null,
      message_type: 'chat'
    }
  ]
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(messages)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #369 chat density toggle', () => {
  test('toggle switches avatar size between 20px (compact) and 28px (comfortable)', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect(win.locator('[data-test="message-avatar"]').first()).toBeVisible({
      timeout: 5_000
    })

    // Read the initial <html data-density> — comfortable by default
    // when no localStorage entry exists.
    const initial = await win.evaluate(
      () => document.documentElement.dataset.density
    )
    expect(initial === 'comfortable' || initial === undefined).toBe(true)

    const avatar = win.locator('[data-test="message-avatar"]').first()
    const comfortableWidth = await avatar.evaluate(
      (el: HTMLElement) => el.getBoundingClientRect().width
    )
    console.log(`#369 comfortable avatar width: ${comfortableWidth}`)
    expect(Math.round(comfortableWidth)).toBe(28)

    // Toggle to compact.
    const toggle = win.locator('[data-test="titlebar-density-toggle"]')
    await expect(toggle).toBeVisible()
    await toggle.click()

    // <html data-density> should flip and the avatar should shrink.
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe('compact')

    const compactWidth = await avatar.evaluate(
      (el: HTMLElement) => el.getBoundingClientRect().width
    )
    console.log(`#369 compact avatar width: ${compactWidth}`)
    expect(Math.round(compactWidth)).toBe(20)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '85-density-compact.png'),
      fullPage: false
    })

    // Toggle back — should restore 28px.
    await toggle.click()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe('comfortable')
    const restoredWidth = await avatar.evaluate(
      (el: HTMLElement) => el.getBoundingClientRect().width
    )
    expect(Math.round(restoredWidth)).toBe(28)

    await app.close()
  })

  test('selection persists across reload via localStorage', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Click compact.
    await win.locator('[data-test="titlebar-density-toggle"]').click()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe('compact')

    // Reload + confirm the preference survived.
    await win.reload()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe('compact')

    await app.close()
  })
})
