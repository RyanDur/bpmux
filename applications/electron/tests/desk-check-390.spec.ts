import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'

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
    mockCurrentIteration('DeskCheck390', '2026-04-23T02:00:00Z'),
    {
      number: 990,
      title: 'Delivered blocked with blocker reference',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'blocked', 'blocked-by:337'],
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: '',
      ...inMockCurrentIteration(1)
    },
    {
      number: 991,
      title: 'Delivered blocked without blocker reference',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'blocked'],
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: '',
      ...inMockCurrentIteration(2)
    },
    {
      number: 992,
      title: 'Delivered unblocked awaiting owner',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: '',
      ...inMockCurrentIteration(3)
    },
    {
      number: 337,
      title: 'Blocker story (drill-in target)',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-23T02:00:00Z',
      body: '',
      ...inMockCurrentIteration(4)
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
  await win.route('**/api/stories/*/timeline', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.reload()
  return { app }
}

const openDelivered = async (board: BoardPage): Promise<void> => {
  await board.openColumn('Acceptance')
}

test.describe('desk-check #390 blocked visual indicator', () => {
  test('blocked label renders distinct badge + tooltip with blocker reference', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    await expect(new ChatPage(win).region()).toBeVisible()
    await openDelivered(board)

    const blocked = board.storyStateBadge(990)
    await expect(blocked).toHaveClass(/story-badge--blocked\b/)
    await expect(blocked).toContainText('blocked')
    await expect(blocked).toHaveAttribute('title', /waiting on #337/)
    await expect(blocked).toHaveAttribute('aria-label', /waiting on #337/)
    await expect(blocked).toHaveAttribute('aria-label', /open blocker/)

    await expect(board.storyRow(990)).toHaveClass(/board-story-row--blocked/)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '99-blocked-badge-with-reference.png'),
      fullPage: true
    })
    await app.close()
  })

  test('blocked without blocked-by falls back to no-reference text — still visible', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    await openDelivered(board)

    const blocked = board.storyStateBadge(991)
    await expect(blocked).toHaveClass(/story-badge--blocked\b/)
    await expect(blocked).toContainText('blocked')
    await expect(blocked).toHaveAttribute('title', /no blocker specified/)

    // Row still carries the dim class — nothing-hidden principle:
    // block-state is visible even when the reference isn't captured.
    await expect(board.storyRow(991)).toHaveClass(/board-story-row--blocked/)

    await app.close()
  })

  test('unblocked delivered row has no blocked class or blocked badge', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    await openDelivered(board)

    const baseline = board.storyStateBadge(992)
    await expect(baseline).toHaveClass(/story-badge--accepted\b/)
    await expect(baseline).toContainText('awaiting owner')

    await expect(board.storyRow(992)).not.toHaveClass(/board-story-row--blocked/)

    await app.close()
  })

  test('click on blocked badge opens the blocker drill-in', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    const drillIn = new DrillInPage(win)
    await openDelivered(board)

    await board.storyStateBadge(990).click()

    // Drill-in for the blocker (#337), not #990 — the click routes
    // through onOpenBlocker with the parsed blocked-by:<N> number.
    const drill = drillIn.panel()
    await expect(drill).toBeVisible()
    await expect(drill).toContainText('#337')

    await app.close()
  })
})
