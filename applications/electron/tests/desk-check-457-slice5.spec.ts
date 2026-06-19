import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

const MOCK_BOARD = {
  fetched_at: '2026-04-26T00:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck457Slice5', '2026-04-26T00:00:00Z'),
    {
      number: 901,
      title: 'Epic: substrate consolidation',
      state: 'in-progress',
      labels: ['Epic'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: ''
    },
    {
      number: 902,
      title: 'Child story under epic — feature',
      state: 'ready',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: '',
      parent_epic: 901,
      ...inMockCurrentIteration(1)
    },
    {
      number: 903,
      title: 'Child story under epic — accepted feature',
      state: 'accepted',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: '',
      parent_epic: 901
    },
    {
      number: 904,
      title: 'Standalone (no epic) story',
      state: 'ready',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: '',
      ...inMockCurrentIteration(2)
    }
  ]
}

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-457-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-457-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #457 slice 5 — Electron Epic-summary section', () => {
  test('#1185 Epic meta-card: progress + breakdown + no nested children + summary toggle-only with body view-details (#1516)', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    await expect(board.region()).toBeVisible()
    await expect(board.tabBar()).toBeVisible()

    await board.openTab('Plan')

    await expect(board.epicsSection()).toBeVisible()

    const epic901 = board.epicCard(901)
    await expect(epic901).toBeVisible()

    const progress = board.epicProgress(901)
    await expect(progress).toBeVisible()
    await expect(progress).toHaveAttribute('value', '1')
    await expect(progress).toHaveAttribute('max', '2')

    await expect(board.epicMiniboard(901)).toBeVisible()
    await expect(board.epicMiniColumn(901, 'Accepted')).toContainText('(1)')
    await expect(board.epicMiniColumn(901, 'Ready')).toContainText('(1)')

    await expect(epic901.locator('.board-epic-children')).toHaveCount(0)
    await expect(epic901.locator('.board-epic-child-state')).toHaveCount(0)
    await expect(epic901.locator('.board-story-row')).toHaveCount(0)

    await expect(epic901.locator('.board-epic-summary button')).toHaveCount(0)
    await expect(board.epicViewDetails(901)).toBeVisible()

    await app.close()
  })

  test('#1260 child story shows ↳ Epic #N link (no full Epic title in visible text) in its state lane', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const board = new BoardPage(win)
    await expect(board.region()).toBeVisible()

    await expect(board.storyRow(902)).toBeVisible()
    const epicRef = board.epicLink(902)
    await expect(epicRef).toHaveText('↳ Epic #901')
    await expect(epicRef).toHaveAttribute(
      'aria-label',
      /open epic #901 Epic: substrate consolidation/
    )

    await expect(board.storyRow(904)).toBeVisible()
    await expect(board.epicLink(904)).toHaveCount(0)

    await app.close()
  })

  test('no Epics section when board has no Epic stories', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-457-empty-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-457-empty-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    await win.route('**/api/board', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          fetched_at: '2026-04-26T00:00:00Z',
          stories: [
            {
              number: 1001,
              title: 'No epic on this board',
              state: 'ready',
              labels: ['Feature'],
              assignees: [],
              created_at: '2026-04-26T00:00:00Z',
              body: ''
            }
          ]
        })
      })
    })
    await win.reload()
    const board = new BoardPage(win)
    await expect(board.tabBar()).toBeVisible()
    await expect(board.epicsSection()).toHaveCount(0)
    await app.close()
  })
})
