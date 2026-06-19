import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// #376: verify color distinguishes awaiting vs accepted at the
// CSS computed-style level. A pure DOM/attribute test passes even
// when the visual colors are identical; asserting getComputedStyle
// catches the "same color by accident" regression Ryan observed in
// img_10.
const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck376', '2026-04-22T16:00:00Z'),
    {
      number: 950,
      title: 'delivered feature (awaiting owner)',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: '',
      ...inMockCurrentIteration(1)
    },
    {
      number: 951,
      title: 'accepted feature (owner-accepted)',
      state: 'accepted',
      labels: ['Feature', 'accepted'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: ''
    },
    {
      number: 952,
      title: 'delivered chore (awaiting team)',
      state: 'delivered',
      labels: ['Chore', 'delivered', 'ready for accept'],
      acceptor: 'team',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: '',
      ...inMockCurrentIteration(2)
    }
  ]
}

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-376-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-376-'))
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
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.route('**/api/stories/*/timeline', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.reload()
  return { app }
}

const GROUP = {
  delivered: { tab: 'Now', label: 'Acceptance' },
  accepted: { tab: 'Done', label: 'Accepted' }
} as const

const openGroup = async (board: BoardPage, state: 'delivered' | 'accepted'): Promise<void> => {
  const { tab, label } = GROUP[state]
  await board.openTab(tab)
  await board.openColumn(label)
}

test.describe('desk-check #376 color-differentiate awaiting vs accepted', () => {
  test('data-state attribute lands on each badge', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    await expect(new ChatPage(win).region()).toBeVisible()

    await openGroup(board, 'delivered')
    await expect(board.storyStateBadge(950)).toHaveAttribute('data-state', 'delivered')
    await expect(board.storyStateBadge(952)).toHaveAttribute('data-state', 'delivered')

    await openGroup(board, 'accepted')
    await expect(board.storyStateBadge(951)).toHaveAttribute('data-state', 'accepted')
    await app.close()
  })

  test('delivered and accepted badges render in different colors', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)

    // Both badges share the same kind ("accepted" = owner-lane
    // outlined); the color axis must differ between states. We
    // read computed styles so "same color" regressions (Ryan's
    // img_10 observation) fail the test — a simpler attribute
    // assertion wouldn't catch a CSS rule deleted.
    await openGroup(board, 'delivered')
    const deliveredColor = await board
      .storyStateBadge(950)
      .evaluate((el: HTMLElement) => getComputedStyle(el).borderTopColor)

    await openGroup(board, 'accepted')
    const acceptedColor = await board
      .storyStateBadge(951)
      .evaluate((el: HTMLElement) => getComputedStyle(el).borderTopColor)

    console.log(`#376 delivered badge border: ${deliveredColor}`)
    console.log(`#376 accepted badge border: ${acceptedColor}`)
    expect(deliveredColor).not.toBe(acceptedColor)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '100-badge-colors-differ.png')
    })
    await app.close()
  })
})
