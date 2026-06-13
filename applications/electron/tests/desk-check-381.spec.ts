import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #381: inline markdown (bold / italic / inline code) renders with
// visual distinction in every markdown-rendered context (chat
// message, drill-in body, drill-in comment). Meyer reset strips
// default inline styling via `font: inherit`; fix scopes restoration
// under `.markdown-body` wrapper that MessageBody now emits.
//
// Observation-falsifier from pickup-gate: "I would reject if
// `**text**` shows without visible weight-difference, `*text*`
// without slant, `` `text` `` without monospace/tinted-background,
// OR if ANY of these fails in chat / drill-in-body / drill-in-
// comment (single MessageBody component — all 3 heal together)."

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MARKDOWN_TEXT = [
  '**bolded-word** plain',
  '*italicized-word* plain',
  'text with `coded-word` in it'
].join(' / ')

const MESSAGES = [
  {
    from: 'designer:claude-code',
    text: MARKDOWN_TEXT,
    ts: '2026-04-22T16:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-381-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-381-'))
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

test.describe('desk-check #381: inline markdown rendering', () => {
  test('strong / em / inline-code each render with distinct computed styles', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect(win.locator('[data-test="message-row"]').first()).toBeVisible({ timeout: 5_000 })

    // Locate the specific inline elements within the rendered
    // markdown. MessageBody wraps in `.markdown-body`.
    const messageBody = win.locator('.markdown-body').first()

    // Bold: computed font-weight should NOT be "400" (normal).
    // Browser default for <strong> is 700 / "bold".
    const strong = messageBody.locator('strong', { hasText: 'bolded-word' }).first()
    await expect(strong).toBeVisible()
    await expect
      .poll(async () =>
        Number(await strong.evaluate((el: HTMLElement) => getComputedStyle(el).fontWeight))
      )
      .toBeGreaterThan(400)

    // Italic: computed font-style should be "italic". Browser
    // default for <em> is italic.
    const em = messageBody.locator('em', { hasText: 'italicized-word' }).first()
    await expect(em).toBeVisible()
    await expect
      .poll(async () => em.evaluate((el: HTMLElement) => getComputedStyle(el).fontStyle))
      .toBe('italic')

    // Inline code: computed font-family should contain a monospace
    // family name. MessageBody.css sets ui-monospace + fallbacks.
    const code = messageBody.locator('code', { hasText: 'coded-word' }).first()
    await expect(code).toBeVisible()
    await expect
      .poll(async () =>
        (await code.evaluate((el: HTMLElement) => getComputedStyle(el).fontFamily)).toLowerCase()
      )
      .toMatch(/mono|menlo|sfmono|courier/)

    // Inline code background: should be NON-transparent (distinct
    // from surrounding text).
    await expect
      .poll(async () => code.evaluate((el: HTMLElement) => getComputedStyle(el).backgroundColor))
      .not.toBe('rgba(0, 0, 0, 0)')

    await win.screenshot({
      path: join(EVIDENCE_DIR, '106-381-markdown-inline.png')
    })
    await app.close()
  })
})
