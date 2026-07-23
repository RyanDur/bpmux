import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import {
  seedCurrentIterationViaRpc,
  addStoryToIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

const EPIC = 1
const CHILD_READY = 2
const CHILD_ACCEPTED = 3
const STANDALONE = 4

const EPIC_TITLE = '[Epic] Epic: substrate consolidation'

const createStory = async (rpc: RpcFn, title: string, parentEpic?: number): Promise<void> => {
  const body: Record<string, unknown> = {
    title,
    user_job: `Fixture story for desk-check #457 slice 5: ${title}.`,
    observation_falsifier: 'reject if the Epic-summary section does not derive from this fixture',
    acceptance_test_path: atpPath(__filename)
  }
  if (parentEpic !== undefined) body.parent_epic = parentEpic
  const resp = await rpc('create_story', body, 'pm')
  expect(resp.status).toBe(200)
}

const moveTo = async (rpc: RpcFn, id: number, state: string): Promise<void> => {
  const resp = await rpc(
    'move_story',
    { id, target_state: state, rationale: `fixture: ${state}` },
    'pm'
  )
  expect(resp.status).toBe(200)
}

const openEpicCard = async (board: BoardPage, epicNumber: number): Promise<void> => {
  const card = board.epicCard(epicNumber)
  if ((await card.getAttribute('open')) === null) {
    await card.locator(':scope > summary').click()
  }
  await expect(card).toHaveAttribute('open', '')
}

const seedEpicFixture = async (result: MockGhLaunchResult): Promise<void> => {
  const { rpc } = result
  await createStory(rpc, EPIC_TITLE)
  await createStory(rpc, 'Child story under epic — feature', EPIC)
  await createStory(rpc, 'Child story under epic — accepted feature', EPIC)
  await createStory(rpc, 'Standalone (no epic) story')

  const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck457Slice5')
  await moveTo(rpc, CHILD_READY, 'ready')
  await moveTo(rpc, CHILD_ACCEPTED, 'accepted')
  await moveTo(rpc, STANDALONE, 'ready')

  await addStoryToIterationViaRpc(rpc, CHILD_READY, iterationId, 1)
  await addStoryToIterationViaRpc(rpc, STANDALONE, iterationId, 2)
}

test.describe('desk-check #457 slice 5 — Electron Epic-summary section', () => {
  test('#1185 Epic meta-card: progress + breakdown + no nested children + summary toggle-only with body view-details (#1516)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedEpicFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect(board.tabBar()).toBeVisible()

      await board.openTab('Plan')

      await expect(board.epicsSection()).toBeVisible()

      const epicCard = board.epicCard(EPIC)
      await expect(epicCard).toBeVisible()

      const progress = board.epicProgress(EPIC)
      await expect(progress).toBeVisible()
      await expect(progress).toHaveAttribute('value', '1')
      await expect(progress).toHaveAttribute('max', '2')

      await openEpicCard(board, EPIC)
      await expect(board.epicMiniboard(EPIC)).toBeVisible()
      await expect(board.epicMiniColumn(EPIC, 'Accepted')).toContainText('(1)')
      await expect(board.epicMiniColumn(EPIC, 'Ready')).toContainText('(1)')

      await expect(epicCard.locator('.board-epic-children')).toHaveCount(0)
      await expect(epicCard.locator('.board-epic-child-state')).toHaveCount(0)
      await expect(epicCard.locator('.board-story-row')).toHaveCount(0)

      await expect(epicCard.locator('.board-epic-summary button')).toHaveCount(0)
      await expect(board.epicViewDetails(EPIC)).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('#1260 child story shows ↳ Epic #N link (no full Epic title in visible text) in its state lane', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedEpicFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.storyRow(CHILD_READY)).toBeVisible()
      const epicRef = board.epicLink(CHILD_READY)
      await expect(epicRef).toHaveText(`↳ Epic #${EPIC}`)
      await expect(epicRef).toHaveAttribute(
        'aria-label',
        new RegExp(`open epic #${EPIC} \\[Epic\\] Epic: substrate consolidation`)
      )

      await expect(board.storyRow(STANDALONE)).toBeVisible()
      await expect(board.epicLink(STANDALONE)).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('no Epics section when board has no Epic stories', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh, rpc } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await createStory(rpc, 'No epic on this board')
      await seedCurrentIterationViaRpc(rpc, 'DeskCheck457Slice5NoEpic')
      await moveTo(rpc, 1, 'ready')
      await win.reload()
      await expect(board.tabBar()).toBeVisible()
      await expect(board.epicsSection()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
