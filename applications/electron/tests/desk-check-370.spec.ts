import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-370-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-370-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-22T16:00:00Z', stories: [] })
    })
  })
  // Chat hydration: one message from designer, plus an activity
  // message so usePresence sees a 'busy' state for designer.
  const messages = [
    {
      from: 'designer:claude-code',
      text: 'hello from the designer',
      ts: '2026-04-22T16:00:00Z',
      to: null,
      addressee: null,
      message_type: 'chat'
    }
  ]
  const activity = [
    {
      from: 'designer:claude-code',
      text: 'busy',
      ts: '2026-04-22T16:00:30Z',
      to: null,
      addressee: null,
      message_type: 'activity'
    }
  ]
  // Route handler distinguishes chat_only=true vs activity-seed.
  await win.route('**/api/messages*', async (route, request) => {
    const url = request.url()
    if (url.includes('message_type=activity')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(activity)
      })
      return
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(messages)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #370 avatar-hover tooltip', () => {
  test('hover avatar reveals role label + description + status', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const avatar = win.locator('[data-test="message-avatar"]').first()
    await expect(avatar).toBeVisible({ timeout: 5_000 })

    // Hover the avatar — CSS :hover reveals the tooltip via
    // opacity transition.
    await avatar.hover()

    const tooltip = win.locator('[data-test="avatar-tooltip"]').first()
    await expect(tooltip).toBeVisible({ timeout: 2_000 })

    // Content assertions — role label, description, current status.
    // Description comes from the static roleInfo constants; status
    // comes from the activity-message seed (busy for designer).
    await expect(tooltip).toContainText('designer')
    await expect(tooltip).toContainText('UX')
    await expect(tooltip.locator('[data-test="avatar-tooltip-status"]'))
      .toContainText(/busy|available|offline|idle/i)

    await avatar.screenshot({
      path: join(EVIDENCE_DIR, '88-avatar-hover-tooltip.png')
    })

    await app.close()
  })

  test('keyboard focus on avatar reveals tooltip (aria-describedby)', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const avatar = win.locator('[data-test="message-avatar"]').first()
    await expect(avatar).toBeVisible({ timeout: 5_000 })

    // Verify aria-describedby wiring.
    const describedby = await avatar.getAttribute('aria-describedby')
    expect(describedby).toBeTruthy()
    expect(describedby).toMatch(/^avatar-info-/)

    // Focus the avatar button and assert the tooltip becomes visible
    // via :focus-within.
    await avatar.focus()
    const tooltip = win.locator('[data-test="avatar-tooltip"]').first()
    await expect(tooltip).toBeVisible({ timeout: 2_000 })

    await app.close()
  })
})
