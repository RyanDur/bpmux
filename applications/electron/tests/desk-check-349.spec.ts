import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MIXED_CLASS_MESSAGES = [
  {
    id: 'm1',
    from: 'designer:claude-code',
    text: 'Plain chat message — no gutter glyph.',
    ts: '2026-04-22T15:00:00Z',
    message_type: 'chat'
  },
  {
    id: 'm2',
    from: 'dev:claude-code',
    text: 'This is an insight — ◊ glyph prefix expected.',
    ts: '2026-04-22T15:01:00Z',
    message_type: 'insight'
  },
  {
    id: 'm3',
    from: 'pm:claude-code',
    text: 'This is an activity message — • glyph expected.',
    ts: '2026-04-22T15:02:00Z',
    message_type: 'activity'
  },
  {
    id: 'm4',
    from: 'ryan:human',
    text: 'This is a system message — ⚙ glyph expected.',
    ts: '2026-04-22T15:03:00Z',
    message_type: 'system'
  },
  {
    id: 'm5',
    from: 'designer:claude-code',
    text: 'Back to chat — no glyph.',
    ts: '2026-04-22T15:04:00Z',
    message_type: 'chat'
  },
  {
    id: 'm6',
    from: 'dev:claude-code',
    text: 'Another insight.',
    ts: '2026-04-22T15:05:00Z',
    message_type: 'insight'
  }
]

const CLASSES = ['chat', 'insight', 'activity', 'system'] as const

async function launchWithFixture(userDataDir?: string): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
  udd: string
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-349-'))
  const udd = userDataDir ?? mkdtempSync(join(tmpdir(), 'bpmux-userdata-349-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MIXED_CLASS_MESSAGES)
    })
  })
  await win.reload()
  return { app, udd }
}

test.describe('desk-check #349 richer filter', () => {
  test('default state: pills present, chat+insight pressed, activity+system hidden', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.waitForTimeout(500)

    // Full region screenshot for pill visibility + default filter state.
    const chatRegion = win.locator('[data-test="chat-region"]')
    await chatRegion.screenshot({
      path: join(EVIDENCE_DIR, '10-filter-default-chat-insight.png')
    })

    // Enumerate pill states via aria-pressed.
    const pillContainer = win.locator('[data-test="filter-pills"]')
    await expect(pillContainer).toBeVisible()

    const allPressed = await win
      .locator('[data-test="filter-pill-all"]')
      .getAttribute('aria-pressed')
    console.log(`#349 AC1/AC2 pill states:`)
    console.log(`  [all] aria-pressed=${allPressed}`)
    for (const cls of CLASSES) {
      const pressed = await win
        .locator(`[data-test="filter-pill-${cls}"]`)
        .getAttribute('aria-pressed')
      console.log(`  [${cls}] aria-pressed=${pressed}`)
    }

    const hiddenCount = await win
      .locator('[data-test="filter-hidden-count"]')
      .textContent()
    console.log(`#349 AC4 hidden-count at default: "${hiddenCount?.trim()}"`)

    await app.close()
  })

  test('toggle system ON: pill becomes pressed, ⚙ glyph renders on system message', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await win.waitForTimeout(500)

    // Click system pill.
    await win.locator('[data-test="filter-pill-system"]').click()
    await win.waitForTimeout(200)

    const pressed = await win
      .locator('[data-test="filter-pill-system"]')
      .getAttribute('aria-pressed')
    console.log(`#349 AC3 after click: [system] aria-pressed=${pressed}`)

    const chatRegion = win.locator('[data-test="chat-region"]')
    await chatRegion.screenshot({
      path: join(EVIDENCE_DIR, '11-filter-system-on-glyphs.png')
    })

    // Enumerate visible rows + extract text to confirm glyphs.
    const messageRows = win.locator('[data-test="message-row"]')
    const rowCount = await messageRows.count()
    console.log(`#349 AC6: ${rowCount} rows rendered with system toggled on:`)
    for (let i = 0; i < rowCount; i++) {
      const text = (await messageRows.nth(i).textContent())?.trim()
      console.log(`  row[${i}] "${(text ?? '').replace(/\n/g, ' ').slice(0, 120)}"`)
    }

    await app.close()
  })

  test('persistence: filter survives relaunch via localStorage', async () => {
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-349-persist-'))
    const first = await launchWithFixture(udd)
    const win1 = await first.app.firstWindow()
    await win1.waitForTimeout(500)

    // Toggle system ON in the first session.
    await win1.locator('[data-test="filter-pill-system"]').click()
    await win1.waitForTimeout(200)
    const preClose = await win1
      .locator('[data-test="filter-pill-system"]')
      .getAttribute('aria-pressed')
    console.log(`#349 AC5 pre-close: [system] aria-pressed=${preClose}`)
    await first.app.close()

    // Relaunch with same user-data-dir — localStorage persists.
    const second = await launchWithFixture(udd)
    const win2 = await second.app.firstWindow()
    await expect(win2.locator('[data-test="chat-region"]')).toBeVisible()
    await win2.waitForTimeout(500)

    const postReopen = await win2
      .locator('[data-test="filter-pill-system"]')
      .getAttribute('aria-pressed')
    console.log(`#349 AC5 post-reopen: [system] aria-pressed=${postReopen}`)

    const pillContainer = win2.locator('[data-test="filter-pills"]')
    await pillContainer.screenshot({
      path: join(EVIDENCE_DIR, '12-filter-persisted-after-reopen.png')
    })

    await second.app.close()
  })

  test('keyboard: pills Tab-reachable', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await win.waitForTimeout(500)

    // Focus the "all" pill + verify subsequent Tabs move through the row.
    await win.locator('[data-test="filter-pill-all"]').focus()
    const order: string[] = []
    const focused = await win.evaluate(
      () => document.activeElement?.getAttribute('data-test') ?? ''
    )
    order.push(focused)
    for (let i = 0; i < 4; i++) {
      await win.keyboard.press('Tab')
      const next = await win.evaluate(
        () => document.activeElement?.getAttribute('data-test') ?? ''
      )
      order.push(next)
    }
    console.log(`#349 AC7 Tab order from all: ${order.join(' → ')}`)

    await app.close()
  })
})
