import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// #390: blocked visual indicator on delivered rows. Fixture carries
//   - #990 blocked + blocked-by:#337  → blocked badge + reference
//   - #991 blocked, no blocked-by     → blocked badge + fallback text
//   - #992 delivered feature no block → baseline awaiting-owner badge
//   - #993 accepted feature + blocked → blocked overrides only on
//                                       delivered; on accepted the
//                                       accept-lane badge still shows
// Distinct values so the negative case (non-blocked row has NO
// blocked class / no blocked badge) is checkable against the baseline.
const MOCK_BOARD = {
  fetched_at: '2026-04-23T02:00:00Z',
  stories: [
    {
      number: 990,
      title: 'Delivered blocked with blocker reference',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'blocked', 'blocked-by:337'],
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: ''
    },
    {
      number: 991,
      title: 'Delivered blocked without blocker reference',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'blocked'],
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: ''
    },
    {
      number: 992,
      title: 'Delivered unblocked awaiting owner',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: ''
    },
    {
      number: 337,
      title: 'Blocker story (drill-in target)',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: ''
    }
  ]
}

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-390-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-390-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  // Match terminal `/api/stories/<n>` only (exclude `/comments` and
  // `/timeline` sub-paths). Registered first so the more-specific
  // sub-path handlers below can't override it for base story fetches.
  await win.route(/\/api\/stories\/\d+$/, async (route) => {
    const url = new URL(route.request().url())
    const match = url.pathname.match(/\/api\/stories\/(\d+)$/)
    const number = match ? Number.parseInt(match[1], 10) : 0
    const story = MOCK_BOARD.stories.find((s) => s.number === number)
    await route.fulfill({
      status: story ? 200 : 404,
      contentType: 'application/json',
      body: JSON.stringify(story ?? {})
    })
  })
  await win.route('**/api/stories/*/comments', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.route('**/api/stories/*/timeline', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.reload()
  return { app }
}

const openDelivered = async (
  win: Awaited<ReturnType<Awaited<ReturnType<typeof electron.launch>>['firstWindow']>>
): Promise<void> => {
  // Post-#516: data-test on summary, not details. See desk-check-375.
  const summary = win.locator('[data-test="board-group-delivered"]').first()
  const isOpen = await summary.evaluate((el: Element) => {
    const details = el.closest('details')
    return details ? (details as HTMLDetailsElement).open : false
  })
  if (!isOpen) {
    await summary.click()
  }
}

test.describe('desk-check #390 blocked visual indicator', () => {
  test('blocked label renders distinct badge + tooltip with blocker reference', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await openDelivered(win)

    const blocked = win.locator('[data-test="board-story-990"] [data-test="story-badge"]')
    await expect(blocked).toHaveAttribute('data-badge-kind', 'blocked')
    await expect(blocked).toContainText('blocked')
    await expect(blocked).toHaveAttribute('title', /waiting on #337/)
    await expect(blocked).toHaveAttribute('aria-label', /waiting on #337/)
    await expect(blocked).toHaveAttribute('aria-label', /open blocker/)

    const row = win.locator('[data-test="board-story-990"]')
    await expect(row).toHaveClass(/board-story-row--blocked/)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '99-blocked-badge-with-reference.png'),
      fullPage: true
    })
    await app.close()
  })

  test('blocked without blocked-by falls back to no-reference text — still visible', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openDelivered(win)

    const blocked = win.locator('[data-test="board-story-991"] [data-test="story-badge"]')
    await expect(blocked).toHaveAttribute('data-badge-kind', 'blocked')
    await expect(blocked).toContainText('blocked')
    await expect(blocked).toHaveAttribute('title', /no blocker specified/)

    // Row still carries the dim class — nothing-hidden principle:
    // block-state is visible even when the reference isn't captured.
    const row = win.locator('[data-test="board-story-991"]')
    await expect(row).toHaveClass(/board-story-row--blocked/)

    await app.close()
  })

  test('unblocked delivered row has no blocked class or blocked badge', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openDelivered(win)

    const baseline = win.locator('[data-test="board-story-992"] [data-test="story-badge"]')
    await expect(baseline).toHaveAttribute('data-badge-kind', 'accepted')
    await expect(baseline).toContainText('awaiting owner')

    const row = win.locator('[data-test="board-story-992"]')
    await expect(row).not.toHaveClass(/board-story-row--blocked/)

    await app.close()
  })

  test('click on blocked badge opens the blocker drill-in', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openDelivered(win)

    const badge = win.locator('[data-test="board-story-990"] [data-test="story-badge"]')
    await badge.click()

    // Drill-in for the blocker (#337), not #990 — the click routes
    // through onOpenBlocker with the parsed blocked-by:<N> number.
    const drill = win.locator('[data-test="board-drill-in"]')
    await expect(drill).toBeVisible()
    await expect(drill).toContainText('#337')

    await app.close()
  })
})
