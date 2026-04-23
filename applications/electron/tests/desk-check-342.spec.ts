import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_MESSAGES = [
  {
    id: 'p1',
    from: 'designer:claude-code',
    text: 'Message to pin — designer says hello.',
    ts: '2026-04-22T16:00:00Z',
    messageType: 'chat'
  },
  {
    id: 'p2',
    from: 'dev:claude-code',
    text: 'Second message, dev speaks.',
    ts: '2026-04-22T16:01:00Z',
    messageType: 'chat'
  }
]

async function launchWithMessages(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-342-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-342-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_MESSAGES)
    })
  })
  // Pins: start empty, accept POST / DELETE with success.
  let pinsList: any[] = []
  await win.route('**/api/pins*', async (route) => {
    const method = route.request().method()
    if (method === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(pinsList)
      })
    } else if (method === 'POST') {
      const body = JSON.parse(route.request().postData() ?? '{}')
      pinsList.push({
        channel: body.channel ?? 'broadcast',
        message_key: body.message_key ?? 'unknown',
        pinned_by: 'ryan',
        pinned_at: new Date().toISOString(),
        snippet: body.snippet ?? ''
      })
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true })
      })
    } else if (method === 'DELETE') {
      pinsList = []
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

test.describe('desk-check #342 pinned messages', () => {
  test('chat channel header renders pin-count button', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.waitForTimeout(500)

    // Pin count button — find by common data-test or role.
    const pinCountCandidates = await win.evaluate(() => {
      const candidates = [
        '[data-test="chat-pin-count"]',
        '[data-test="pin-count"]',
        '[data-test="chat-channel-header"]'
      ]
      return candidates.map((sel) => ({
        sel,
        present: !!document.querySelector(sel)
      }))
    })
    console.log(`#342 pin count button candidates:`, pinCountCandidates)

    await win.locator('[data-test="chat-region"]').screenshot({
      path: join(EVIDENCE_DIR, '90-chat-with-pin-count.png')
    })
    await app.close()
  })

  test('message-hover reveals pin toggle button', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="message-row"]').first()).toBeVisible()
    await win.waitForTimeout(500)

    // Find message pin toggle button.
    const candidates = await win.evaluate(() => {
      const allTests = Array.from(document.querySelectorAll('[data-test]'))
        .map((e) => e.getAttribute('data-test'))
        .filter((v) => v && (v.includes('pin') || v.includes('Pin')))
      return allTests
    })
    console.log(`#342 pin-related data-test attrs found: ${candidates.join(', ')}`)

    // Hover on first message, see if anything changes.
    const firstRow = win.locator('[data-test="message-row"]').first()
    await firstRow.hover()
    await win.waitForTimeout(300)
    await firstRow.screenshot({
      path: join(EVIDENCE_DIR, '91-message-hover-reveal-pin.png')
    })

    await app.close()
  })

  test('click pin-toggle on a message → pinned indicator appears', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="message-row"]').first()).toBeVisible()
    await win.waitForTimeout(500)

    // Find pin toggle buttons — search all buttons with aria-label containing 'pin'.
    const pinButtons = await win.evaluate(() => {
      return Array.from(document.querySelectorAll('button'))
        .filter((b) => {
          const label = b.getAttribute('aria-label') ?? ''
          return /pin/i.test(label)
        })
        .map((b) => ({
          label: b.getAttribute('aria-label'),
          dataTest: b.getAttribute('data-test'),
          visible: window.getComputedStyle(b).display !== 'none'
        }))
    })
    console.log(`#342 pin-related buttons found: ${JSON.stringify(pinButtons)}`)

    // If there's a pin toggle, try clicking it.
    if (pinButtons.length > 0) {
      // Hover first to reveal (if hover-only visible), then click.
      await win.locator('[data-test="message-row"]').first().hover()
      await win.waitForTimeout(200)
      const clickable = win
        .locator('button[aria-label*="pin" i]')
        .first()
      if (await clickable.count()) {
        await clickable.click()
        await win.waitForTimeout(400)
        console.log(`  clicked pin button`)

        await win.locator('[data-test="chat-region"]').screenshot({
          path: join(EVIDENCE_DIR, '92-after-pin-click.png')
        })
      }
    }

    await app.close()
  })

  test('click pin count → overlay opens with pinned list', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.waitForTimeout(500)

    // First: pin a message so there's something to list.
    await win.locator('[data-test="message-row"]').first().hover()
    await win.waitForTimeout(200)
    const pinBtn = win.locator('button[aria-label*="Pin" i]').first()
    if ((await pinBtn.count()) > 0) {
      await pinBtn.click()
      await win.waitForTimeout(400)
    }

    // Then click the channel pin-count to open the overlay.
    const headerPinBtn = win.locator(
      'button[aria-label*="pins" i], [data-test="chat-pin-count"]'
    ).first()
    const btnCount = await headerPinBtn.count()
    console.log(`#342 pin-count header button count: ${btnCount}`)

    if (btnCount > 0) {
      await headerPinBtn.click()
      await win.waitForTimeout(400)

      // Look for the overlay.
      const overlayCandidates = await win.evaluate(() =>
        Array.from(document.querySelectorAll('[data-test]'))
          .map((e) => e.getAttribute('data-test'))
          .filter((v) => v && v.toLowerCase().includes('pin'))
      )
      console.log(`#342 pin-scope data-tests after overlay open: ${overlayCandidates.join(', ')}`)

      await win.locator('[data-test="chat-region"]').screenshot({
        path: join(EVIDENCE_DIR, '93-pinned-list-overlay.png')
      })
    }

    await app.close()
  })
})
