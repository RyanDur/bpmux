import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import {
  seedCurrentIterationViaRpc,
  addStoryToIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

const createStory = async (rpc: RpcFn, title: string, parentEpic?: number): Promise<void> => {
  const body: Record<string, unknown> = {
    title,
    user_job: `Fixture story for desk-check #457 slice 6: ${title}.`,
    observation_falsifier:
      'reject if the Epic state-derivation does not match the #1450 truth-table',
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

const collectConsoleErrors = (result: MockGhLaunchResult): string[] => {
  const consoleErrors: string[] = []
  result.win.on('console', (m) => {
    if (m.type() === 'error') consoleErrors.push(m.text())
  })
  result.win.on('pageerror', (e) => consoleErrors.push(String(e)))
  return consoleErrors
}

test.describe('desk-check #457 slice 6 — Epic hybrid state-derivation + lane filter', () => {
  test('Epic with in-progress child derives in-progress state', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh, rpc } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      const consoleErrors = collectConsoleErrors(result)

      await createStory(rpc, '[Epic] Epic with in-progress work')
      await createStory(rpc, 'Child in progress', 1)
      await moveTo(rpc, 2, 'in-progress')

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.epicState(1)).toHaveText('in-progress')
      expect(consoleErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Epic with all-delivered children derives in-progress (delivered is not a terminal child state per #1450 truth-table)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh, rpc } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      const consoleErrors = collectConsoleErrors(result)

      await createStory(rpc, '[Epic] Epic ready for owner')
      await createStory(rpc, 'Child A delivered', 1)
      await createStory(rpc, 'Child B delivered', 1)
      await moveTo(rpc, 2, 'acceptance')
      await moveTo(rpc, 3, 'acceptance')

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.epicState(1)).toHaveText('in-progress')
      expect(consoleErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Epic explicit state is ignored; in-progress child derives in-progress per #1450 truth-table', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh, rpc } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      const consoleErrors = collectConsoleErrors(result)

      await createStory(rpc, '[Epic] Owner-accepted epic')
      await createStory(rpc, 'Late-arriving child', 1)
      await moveTo(rpc, 1, 'accepted')
      await moveTo(rpc, 2, 'in-progress')

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.epicState(1)).toHaveText('in-progress')
      expect(consoleErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Epic story is filtered out of state lanes (lives only in Epics section)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh, rpc } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      const consoleErrors = collectConsoleErrors(result)

      await createStory(rpc, '[Epic] Lone epic in ready')
      await createStory(rpc, 'Plain ready feature')
      const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck457Slice6')
      await moveTo(rpc, 1, 'ready')
      await moveTo(rpc, 2, 'ready')

      await addStoryToIterationViaRpc(rpc, 2, iterationId, 1)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect(board.storyRow(2)).toBeVisible()
      await expect(board.storyNavigate(1)).toHaveCount(0)
      await board.openTab('Plan')
      await expect(board.epicCard(1)).toBeVisible()
      expect(consoleErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
