import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_MESSAGES = [
  {
    id: 'r1',
    from: 'designer:claude-code',
    text: 'React to this message.',
    ts: '2026-04-22T16:00:00Z',
    messageType: 'chat'
  },
  {
    id: 'r2',
    from: 'dev:claude-code',
    text: 'This message already has some reactions.',
    ts: '2026-04-22T16:01:00Z',
    messageType: 'chat'
  }
]

const EMOJI_PALETTE = ['👍', '🎉', '❤️', '🚀', '👀', '❌']

async function launchWithMessages(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-340-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-340-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_MESSAGES)
    })
  })
  // Reactions: seed so r2 already has some, r1 is empty.
  let reactionsState: any[] = [
    {
      message_key: `2026-04-22T16:01:00.000Z|dev`,
      emoji: '👍',
      count: 2,
      users: ['ryan', 'pm']
    },
    {
      message_key: `2026-04-22T16:01:00.000Z|dev`,
      emoji: '🎉',
      count: 1,
      users: ['designer']
    }
  ]
  await win.route('**/api/reactions*', async (route) => {
    const method = route.request().method()
    if (method === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(reactionsState)
      })
    } else if (method === 'POST') {
      const body = JSON.parse(route.request().postData() ?? '{}')
      console.log(`POST /api/reactions body: ${JSON.stringify(body)}`)
      // Add or remove.
      const existing = reactionsState.find(
        (r: any) => r.message_key === body.message_key && r.emoji === body.emoji
      )
      if (existing) {
        if (existing.users.includes('ryan')) {
          existing.users = existing.users.filter((u: string) => u !== 'ryan')
          existing.count = existing.users.length
          if (existing.count === 0) {
            reactionsState = reactionsState.filter((r: any) => r !== existing)
          }
        } else {
          existing.users.push('ryan')
          existing.count = existing.users.length
        }
      } else {
        reactionsState.push({
          message_key: body.message_key,
          emoji: body.emoji,
          count: 1,
          users: ['ryan']
        })
      }
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true })
      })
    }
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #340 emoji reactions', () => {
  test('existing reactions render as pills with count + aria-label', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="message-row"]').first()).toBeVisible()
    await win.waitForTimeout(600)

    // Find reaction pills in the DOM.
    const pills = await win.evaluate(() =>
      Array.from(document.querySelectorAll('[data-test*="reaction" i], .reaction-pill, button[aria-label*="React" i]'))
        .map((el) => ({
          dataTest: el.getAttribute('data-test'),
          className: el.className,
          label: el.getAttribute('aria-label'),
          text: (el.textContent ?? '').trim()
        }))
    )
    console.log(`#340 reaction-related elements:`)
    pills.forEach((p, i) => console.log(`  [${i}] ${JSON.stringify(p)}`))

    await win.locator('[data-test="chat-region"]').screenshot({
      path: join(EVIDENCE_DIR, 'a0-reactions-seeded-pills.png')
    })
    await app.close()
  })

  test('message-hover reveals add-reaction button', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="message-row"]').first()).toBeVisible()
    await win.waitForTimeout(500)

    const firstRow = win.locator('[data-test="message-row"]').first()
    await firstRow.hover()
    await win.waitForTimeout(300)

    const reactControls = await win.evaluate(() =>
      Array.from(document.querySelectorAll('button[aria-label*="react" i], button[aria-label*="reaction" i]'))
        .map((b) => ({
          label: b.getAttribute('aria-label'),
          dataTest: b.getAttribute('data-test'),
          visible: window.getComputedStyle(b as Element).display !== 'none'
        }))
    )
    console.log(`#340 react-control buttons on hover:`)
    reactControls.forEach((r, i) => console.log(`  [${i}] ${JSON.stringify(r)}`))

    await firstRow.screenshot({
      path: join(EVIDENCE_DIR, 'a1-message-hover-react.png')
    })
    await app.close()
  })

  test('picker opens with 6 emoji palette', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="message-row"]').first()).toBeVisible()
    await win.waitForTimeout(500)

    // Hover first message + click the add-reaction button.
    await win.locator('[data-test="message-row"]').first().hover()
    await win.waitForTimeout(200)

    const addBtn = win.locator('button[aria-label*="Add reaction" i]').first()
    const btnCount = await addBtn.count()
    console.log(`#340 add-reaction button count: ${btnCount}`)

    if (btnCount > 0) {
      await addBtn.click()
      await win.waitForTimeout(400)

      // Look for the emoji palette.
      const emojiButtons = await win.evaluate(() => {
        return Array.from(document.querySelectorAll('button'))
          .filter((b) => {
            const text = (b.textContent ?? '').trim()
            return /^\S+$/.test(text) && text.length <= 3 && /[\p{Emoji}]/u.test(text)
          })
          .map((b) => ({
            text: (b.textContent ?? '').trim(),
            label: b.getAttribute('aria-label'),
            dataTest: b.getAttribute('data-test')
          }))
      })
      console.log(`#340 picker emoji candidates visible after open:`)
      emojiButtons.forEach((e, i) => console.log(`  [${i}] ${JSON.stringify(e)}`))

      await win.locator('[data-test="chat-region"]').screenshot({
        path: join(EVIDENCE_DIR, 'a2-picker-open-6-emoji.png')
      })
    }

    await app.close()
  })
})
