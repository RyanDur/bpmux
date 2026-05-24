import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Fixture board covers the three badge variants: team-accept lane
// (chore in delivered state), owner-accept lane (feature in delivered
// state), rework cycle (any type with desk-check-fail label present).
const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    {
      number: 800,
      title: 'Team-accepted chore',
      state: 'delivered',
      labels: ['Chore', 'delivered', 'ready for accept'],
      acceptor: 'team',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Chore body.'
    },
    {
      number: 801,
      title: 'Owner-accept feature',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Feature body.'
    },
    {
      number: 802,
      title: 'Rework cycle story',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'desk-check-fail'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Rework body.'
    },
    {
      number: 803,
      title: 'Plain ready — no badge',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Upstream state.'
    }
  ]
}

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-368-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-368-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/*/comments', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: '[]'
    })
  })
  await win.route('**/api/stories/*/timeline', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: '[]'
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #368 team-accept badge', () => {
  test('team-accept / accept / rework badges render on expected stories', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Post-#516: data-test on summary, not details. Click summary
    // directly to fire native <details> toggle. See desk-check-375.
    const deliveredSummary = win.locator('[data-test="board-group-delivered"]').first()
    const isOpen = await deliveredSummary.evaluate((el: Element) => {
      const details = el.closest('details')
      return details ? (details as HTMLDetailsElement).open : false
    })
    if (!isOpen) {
      await deliveredSummary.click()
    }

    const chore = win.locator('[data-test="board-story-800"]')
    await expect(chore).toBeVisible()
    await expect(chore.locator('[data-test="story-badge"]')).toHaveAttribute(
      'data-badge-kind',
      'team-accepted'
    )
    // #375 state-aware label: delivered + team lane reads
    // "awaiting team" (tense-correct for pre-accept state).
    await expect(chore.locator('[data-test="story-badge"]')).toContainText('awaiting team')

    const feature = win.locator('[data-test="board-story-801"]')
    await expect(feature.locator('[data-test="story-badge"]')).toHaveAttribute(
      'data-badge-kind',
      'accepted'
    )
    // #375 state-aware label: delivered + owner lane reads
    // "awaiting owner" (not "accepted" — that's past-tense wrong
    // for a row the owner hasn't accepted yet).
    await expect(feature.locator('[data-test="story-badge"]')).toContainText('awaiting owner')

    const rework = win.locator('[data-test="board-story-802"]')
    await expect(rework.locator('[data-test="story-badge"]')).toHaveAttribute(
      'data-badge-kind',
      'rework'
    )
    await expect(rework.locator('[data-test="story-badge"]')).toContainText('rework')

    // Upstream states should NOT carry a badge.
    const plain = win.locator('[data-test="board-story-803"]')
    await expect(plain.locator('[data-test="story-badge"]')).toHaveCount(0)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '90-badges-all-variants.png'),
      fullPage: true
    })

    await app.close()
  })

  test('clicking badge opens drill-in and auto-expands timeline', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const deliveredSummary = win.locator('[data-test="board-group-delivered"]').first()
    const isOpen = await deliveredSummary.evaluate((el: Element) => {
      const details = el.closest('details')
      return details ? (details as HTMLDetailsElement).open : false
    })
    if (!isOpen) {
      await deliveredSummary.click()
    }

    const badge = win.locator('[data-test="board-story-800"] [data-test="story-badge"]').first()
    await badge.click()

    // Drill-in should be open.
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

    // Timeline details should be auto-expanded.
    const timeline = win.locator('[data-test="drill-in-timeline"]')
    await expect(timeline).toBeVisible()
    await expect(timeline).toHaveAttribute('open', '')

    await app.close()
  })
})
