import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import {
  addStoryToIterationViaRpc,
  seedCurrentIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'

const READY_TITLES = ['First ready story', 'Second ready story', 'Third ready story']

const seedReadyMembers = async (rpc: RpcFn): Promise<void> => {
  for (const title of READY_TITLES) {
    const createResp = await rpc(
      'create_story',
      {
        title: `[Feature] ${title}`,
        user_job: 'When the human reorders a Ready row, the order persists through refetch.',
        observation_falsifier: 'reject if the reordered order snaps back after board refetch',
        acceptance_test_path: 'applications/electron/tests/desk-check-333.spec.ts'
      },
      'pm'
    )
    expect(createResp.status).toBe(200)
  }
  const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck333')
  for (const id of [1, 2, 3]) {
    const moveResp = await rpc(
      'move_story',
      { id, target_state: 'ready', rationale: 'desk-check-333 reorder fixture' },
      'pm'
    )
    expect(moveResp.status).toBe(200)
  }
  await addStoryToIterationViaRpc(rpc, 1, iterationId, 1)
  await addStoryToIterationViaRpc(rpc, 2, iterationId, 2)
  await addStoryToIterationViaRpc(rpc, 3, iterationId, 3)
}

test.describe('desk-check #333 drag-to-reorder (rework)', () => {
  test('dragged row hides via data-dragging during gesture', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      await expect(board.group('Ready')).toBeVisible()

      const row = board.storyRow(1)
      const grip = board.storyGrip(1)
      await expect(row).toBeVisible()
      await expect(grip).toBeVisible()
      await expect(row).not.toHaveAttribute('data-dragging', 'true')

      await grip.evaluate((el) => {
        const dt = new DataTransfer()
        el.dispatchEvent(new DragEvent('dragstart', { bubbles: true, dataTransfer: dt }))
      })
      await expect(row).toHaveAttribute('data-dragging', 'true')

      await grip.evaluate((el) => {
        el.dispatchEvent(new DragEvent('dragend', { bubbles: true }))
      })
      await expect(row).not.toHaveAttribute('data-dragging', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('reorder POST fires + optimistic order persists through refetch', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)

      await expect(async () => {
        expect(await board.storyOrder('Ready')).toEqual([1, 2, 3])
      }).toPass({ timeout: 5_000 })

      const firstRow = board.storyNavigate(1)
      await firstRow.focus()
      await firstRow.press('Control+Shift+ArrowDown')

      await expect(async () => {
        expect(await board.storyOrder('Ready')).not.toEqual([1, 2, 3])
      }).toPass({ timeout: 5_000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('#1524 a single real drag dropped ON the immediate-below row swaps them', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      await expect(async () => {
        expect(await board.storyOrder('Ready')).toEqual([1, 2, 3])
      }).toPass({ timeout: 5_000 })

      await board.storyGrip(1).dragTo(board.storyRow(2))

      await expect(async () => {
        expect(await board.storyOrder('Ready')).toEqual([2, 1, 3])
      }).toPass({ timeout: 5_000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
