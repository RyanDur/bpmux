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


const launchWithFixture = async (
  userDataDir?: string
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
  udd: string
}> => {
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
  test('default state: 3 pills present, chat on, events + bpmux muted', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const pillContainer = win.locator('[data-test="filter-pills"]')
    await expect(pillContainer).toBeVisible()

    await expect(win.locator('[data-test="filter-pill-chat"]')).toHaveAttribute('data-state', 'on')
    await expect(win.locator('[data-test="filter-pill-events"]')).toHaveAttribute(
      'data-state',
      'off'
    )
    await expect(win.locator('[data-test="filter-pill-bpmux"]')).toHaveAttribute('data-state', 'off')

    await pillContainer.screenshot({
      path: join(EVIDENCE_DIR, '10-filter-default-chat-only.png')
    })

    await app.close()
  })

  test('toggle bpmux ON: pill becomes active', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const bpmuxPill = win.locator('[data-test="filter-pill-bpmux"]')
    await expect(bpmuxPill).toHaveAttribute('data-state', 'off')
    await bpmuxPill.click()
    await expect(bpmuxPill).toHaveAttribute('data-state', 'on')

    await win.locator('[data-test="chat-region"]').screenshot({
      path: join(EVIDENCE_DIR, '11-filter-bpmux-on.png')
    })

    await app.close()
  })

  test('persistence: filter survives relaunch via localStorage', async () => {
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-349-persist-'))
    const first = await launchWithFixture(udd)
    const win1 = await first.app.firstWindow()
    await expect(win1.locator('[data-test="chat-region"]')).toBeVisible()

    const bpmuxPill1 = win1.locator('[data-test="filter-pill-bpmux"]')
    await bpmuxPill1.click()
    await expect(bpmuxPill1).toHaveAttribute('data-state', 'on')
    await first.app.close()

    const second = await launchWithFixture(udd)
    const win2 = await second.app.firstWindow()
    await expect(win2.locator('[data-test="chat-region"]')).toBeVisible()

    await expect(win2.locator('[data-test="filter-pill-bpmux"]')).toHaveAttribute(
      'data-state',
      'on'
    )

    await win2.locator('[data-test="filter-pills"]').screenshot({
      path: join(EVIDENCE_DIR, '12-filter-persisted-after-reopen.png')
    })

    await second.app.close()
  })

  test('keyboard: pills Tab-reachable', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await win.locator('[data-test="filter-pill-chat"] input').focus()
    const reached: string[] = []
    for (let i = 0; i < 3; i++) {
      const cls = await win.evaluate(
        () =>
          document.activeElement
            ?.closest('[data-test^="filter-pill-"]')
            ?.getAttribute('data-test') ?? ''
      )
      reached.push(cls)
      await win.keyboard.press('Tab')
    }
    expect(reached).toContain('filter-pill-chat')
    expect(reached).toContain('filter-pill-events')
    expect(reached).toContain('filter-pill-bpmux')

    await app.close()
  })
})
