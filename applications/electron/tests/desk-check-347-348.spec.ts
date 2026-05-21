import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_MESSAGES = [
  {
    id: 'm1',
    from: 'designer:claude-code',
    text: 'First message from designer — avatar + role name visible here.',
    ts: '2026-04-22T15:00:00Z',
    messageType: 'chat'
  },
  {
    id: 'm2',
    from: 'designer:claude-code',
    text: 'Second message from same sender within 5 minutes — should group (no avatar, no header).',
    ts: '2026-04-22T15:02:00Z',
    messageType: 'chat'
  },
  {
    id: 'm3',
    from: 'designer:claude-code',
    text: 'Third consecutive — still grouped.',
    ts: '2026-04-22T15:03:30Z',
    messageType: 'chat'
  },
  {
    id: 'm4',
    from: 'dev:claude-code',
    text: 'Dev speaks — breaks the group, new avatar + header.',
    ts: '2026-04-22T15:05:00Z',
    messageType: 'chat'
  },
  {
    id: 'm5',
    from: 'pm:claude-code',
    text: 'PM addressing ryan directly — should get left-border accent.',
    ts: '2026-04-22T15:06:00Z',
    messageType: 'chat',
    addressee: 'ryan'
  },
  {
    id: 'm6',
    from: 'ryan:human',
    text: 'Ryan speaks — filled circle avatar instead of rounded square.',
    ts: '2026-04-22T15:07:00Z',
    messageType: 'chat'
  },
  {
    id: 'm7',
    from: 'dev:claude-code',
    text: 'Message addressed to pm (not me) — body should be dimmed.',
    ts: '2026-04-22T15:08:00Z',
    messageType: 'chat',
    addressee: 'pm'
  }
]

const launchAndSeed = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-'))
  const { app } = await launchIsolatedElectron({ sessionBase })
  const win = await app.firstWindow()
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_MESSAGES)
    })
  })
  // Trigger a reload so the fixture is consumed on the first fetch.
  await win.reload()
  return { app }
}

test.describe('desk-check 347+348', () => {
  test('capture full-window state with seeded messages', async () => {
    const { app } = await launchAndSeed()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.waitForTimeout(500)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '01-full-window.png'),
      fullPage: false
    })
    await app.close()
  })

  test('capture chat-region with avatars + grouping + addressing visible', async () => {
    const { app } = await launchAndSeed()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.waitForTimeout(500)
    const chatRegion = win.locator('[data-test="chat-region"]')
    await chatRegion.screenshot({
      path: join(EVIDENCE_DIR, '02-chat-avatars-grouping-addressing.png')
    })

    const avatars = win.locator('[data-test="message-avatar"]')
    const avatarCount = await avatars.count()
    console.log(
      `#348 AC1+AC2: ${avatarCount} avatars render for 7 messages (designer-grouped = 1 anchor; dev + pm + ryan + dev(to-pm) = 4 more ⇒ expect 5)`
    )

    for (let i = 0; i < avatarCount; i++) {
      const role = await avatars.nth(i).getAttribute('data-role')
      const classList = await avatars.nth(i).evaluate((el) => Array.from(el.classList))
      console.log(`  avatar[${i}] role=${role} classes=${classList.join(' ')}`)
    }

    await app.close()
  })

  test('capture i-search failing state with ✗ glyph', async () => {
    const { app } = await launchAndSeed()
    const win = await app.firstWindow()
    await win.waitForTimeout(500)
    const textarea = win.locator('[data-test="chat-input-textarea"]')
    await textarea.focus()
    await textarea.press('Control+r')
    await win.keyboard.type('zzznevermatchesxyzzy')
    const strip = win.locator('[data-test="chat-input-isearch"]')
    await expect(strip).toBeVisible({ timeout: 2000 })
    const stripText = await strip.textContent()
    console.log(`#347 AC7 i-search failing strip text: "${stripText?.trim()}"`)
    await strip.screenshot({
      path: join(EVIDENCE_DIR, '03-isearch-failing.png')
    })
    await app.close()
  })

  test('dump accessibility tree + computed a11y substrate props', async () => {
    const { app } = await launchAndSeed()
    const win = await app.firstWindow()
    await win.waitForTimeout(500)

    const messageListRole = await win.locator('[data-test="message-list"]').getAttribute('role')
    const messageListAriaLive = await win
      .locator('[data-test="message-list"]')
      .getAttribute('aria-live')
    const messageListAriaLabel = await win
      .locator('[data-test="message-list"]')
      .getAttribute('aria-label')

    console.log(`#347 AC4 message-list ARIA:`)
    console.log(`  role=${messageListRole}`)
    console.log(`  aria-live=${messageListAriaLive}`)
    console.log(`  aria-label=${messageListAriaLabel}`)

    const fgDimPx = await win.evaluate(() => {
      return window.getComputedStyle(document.documentElement).getPropertyValue('--fg-dim').trim()
    })
    const bgBasePx = await win.evaluate(() => {
      return window.getComputedStyle(document.documentElement).getPropertyValue('--bg-base').trim()
    })
    console.log(`#347 AC1 tokens: --fg-dim=${fgDimPx} on --bg-base=${bgBasePx}`)

    const fontFamily = await win.evaluate(() => {
      return window.getComputedStyle(document.body).fontFamily
    })
    console.log(`#347 AC3 body font-family: ${fontFamily}`)

    const srOnlySenders = await win.locator('.sr-only').count()
    console.log(`#347 AC5 sr-only sender prefixes: ${srOnlySenders} found`)

    await app.close()
  })
})
