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

// Fixture board covers the three badge variants: team-accept lane
// (chore in delivered state), owner-accept lane (feature in delivered
// state), rework cycle (any type with desk-check-fail label present).
const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck368', '2026-04-22T16:00:00Z'),
    {
      number: 800,
      title: 'Team-accepted chore',
      state: 'delivered',
      labels: ['Chore', 'delivered', 'ready for accept'],
      acceptor: 'team',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Chore body.',
      ...inMockCurrentIteration(1)
    },
    {
      number: 801,
      title: 'Owner-accept feature',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Feature body.',
      ...inMockCurrentIteration(2)
    },
    {
      number: 802,
      title: 'Rework cycle story',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'desk-check-fail'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Rework body.',
      ...inMockCurrentIteration(3)
    },
    {
      number: 803,
      title: 'Plain ready — no badge',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Upstream state.',
      ...inMockCurrentIteration(4)
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
    const board = new BoardPage(win)
    await expect(new ChatPage(win).region()).toBeVisible()

    await board.openColumn('Acceptance')

    const chore = board.storyStateBadge(800)
    await expect(chore).toBeVisible()
    await expect(chore).toHaveClass(/story-badge--team-accepted\b/)
    await expect(chore).toContainText('awaiting team')

    const feature = board.storyStateBadge(801)
    await expect(feature).toHaveClass(/story-badge--accepted\b/)
    await expect(feature).toContainText('awaiting owner')

    const rework = board.storyStateBadge(802)
    await expect(rework).toHaveClass(/story-badge--rework\b/)
    await expect(rework).toContainText('rework')

    const plain = board.storyStateBadge(803)
    await expect(plain).toHaveCount(0)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '90-badges-all-variants.png'),
      fullPage: true
    })

    await app.close()
  })

  test('clicking badge opens drill-in and auto-expands timeline', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    const drillIn = new DrillInPage(win)
    await board.openColumn('Acceptance')

    await board.storyStateBadge(800).click()

    await expect(drillIn.panel()).toBeVisible()

    const timeline = drillIn.timelineSection()
    await expect(timeline).toBeVisible()
    await expect(timeline).toHaveAttribute('open', '')

    await app.close()
  })
})
