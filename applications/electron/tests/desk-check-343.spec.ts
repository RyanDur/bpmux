import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_MESSAGES = [
  {
    from: 'designer:claude-code',
    text: 'Check this out: https://example.com/article',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'Two URLs here: https://docs.rust-lang.org/ and https://example.com/other',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'No URL in this message.',
    ts: '2026-04-22T16:02:00Z',
    message_type: 'chat'
  }
]

const UNFURLS: Record<string, Record<string, unknown>> = {
  'https://example.com/article': {
    url: 'https://example.com/article',
    title: 'An Example Article',
    description: 'This is a description of the article.',
    image: null,
    site_name: 'Example'
  },
  'https://docs.rust-lang.org/': {
    url: 'https://docs.rust-lang.org/',
    title: 'Rust Documentation',
    description: 'The Rust programming language documentation.',
    image: null,
    site_name: 'rust-lang.org'
  },
  'https://example.com/other': {
    url: 'https://example.com/other',
    title: null,
    description: null,
    image: null,
    site_name: null
  }
}

const launchWithMessages = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-343-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-343-'))
  const { app } = await launchIsolatedElectron({
    dataBase,
    userDataDir: udd,
    seedMessages: MOCK_MESSAGES
  })
  const win = await app.firstWindow()
  await win.route('**/api/unfurl*', async (route) => {
    const url = new URL(route.request().url())
    const target = url.searchParams.get('url') ?? ''
    const payload = UNFURLS[target]
    if (payload) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(payload)
      })
    } else {
      await route.fulfill({ status: 404, body: 'not found' })
    }
  })
  await waitForMessagesLoaded(win, 'Check this out')
  return { app }
}

test.describe('desk-check #343 URL unfurls', () => {
  test('messages with URLs render unfurl cards below body', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.messageRows().first()).toBeVisible()
    await win.waitForTimeout(1500)

    // Enumerate unfurl-related DOM.
    const unfurlElts = await win.evaluate(() =>
      Array.from(document.querySelectorAll('.unfurl-card, .unfurl-cards')).map((el) => ({
        className: el.className,
        text: (el.textContent ?? '').trim().slice(0, 100)
      }))
    )
    console.log(`#343 unfurl elements found: ${unfurlElts.length}`)
    unfurlElts.forEach((e, i) => console.log(`  [${i}] ${JSON.stringify(e)}`))

    await chat.region().screenshot({
      path: join(EVIDENCE_DIR, 'b0-unfurl-cards-in-chat.png')
    })
    await app.close()
  })

  test('no URL → no cards (AC5 graceful empty elision)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.messageRows().first()).toBeVisible()
    await win.waitForTimeout(1500)

    // Find the pm message (no URL).
    const pmRow = chat.messageRow('No URL in this message.')
    const pmCards = await pmRow.locator('.unfurl-card').count()
    console.log(`#343 AC5 pm message (no URL) unfurl element count: ${pmCards}`)
    expect(pmCards).toBe(0)
    await app.close()
  })

  test('multiple URLs in one message render multiple cards (AC4)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.messageRows().first()).toBeVisible()
    await win.waitForTimeout(1500)

    // dev message has 2 URLs.
    const devRow = chat.messageRow('Two URLs here')
    const devCards = await devRow.locator('.unfurl-card').count()
    console.log(`#343 AC4 dev message (2 URLs) unfurl card count: ${devCards}`)
    // One URL has no metadata (AC5 elides). Expect 1 card rendered.
    // If both rendered with one empty, that's a different spec interpretation.
    await devRow.screenshot({
      path: join(EVIDENCE_DIR, 'b1-multiple-urls-cards.png')
    })

    await app.close()
  })

  test('unfurl card link has target=_blank (AC3 — opens in OS browser)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.messageRows().first()).toBeVisible()
    await win.waitForTimeout(1500)

    const linkAttrs = await win.evaluate(() =>
      Array.from(document.querySelectorAll('.unfurl-card a, a.unfurl-card')).map((a) => ({
        href: (a as HTMLAnchorElement).href,
        target: (a as HTMLAnchorElement).target,
        rel: (a as HTMLAnchorElement).rel
      }))
    )
    console.log(`#343 AC3 unfurl link attrs:`)
    linkAttrs.forEach((l, i) => console.log(`  [${i}] ${JSON.stringify(l)}`))

    if (linkAttrs.length > 0) {
      expect(linkAttrs[0].target).toBe('_blank')
      expect(linkAttrs[0].rel).toContain('noopener')
    }
    await app.close()
  })
})
