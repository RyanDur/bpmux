import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
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
        user_job: 'When the human keyboard-reorders a Ready row, it moves + announces position.',
        observation_falsifier:
          'reject if Ctrl+Shift+Up/Down does not move the focused Ready row or announce position',
        acceptance_test_path: atpPath(__filename)
      },
      'pm'
    )
    expect(createResp.status).toBe(200)
  }
  const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck357')
  for (const id of [1, 2, 3]) {
    const moveResp = await rpc(
      'move_story',
      { id, target_state: 'ready', rationale: 'keyboard-reorder fixture' },
      'pm'
    )
    expect(moveResp.status).toBe(200)
  }
  await addStoryToIterationViaRpc(rpc, 1, iterationId, 1)
  await addStoryToIterationViaRpc(rpc, 2, iterationId, 2)
  await addStoryToIterationViaRpc(rpc, 3, iterationId, 3)
}

test.describe('desk-check #357 keyboard-accessible drag-reorder', () => {
  test('Ctrl+Shift+Down on focused row moves story + announces position', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      await expect(board.group('Ready')).toBeVisible()

      const firstRow = board.storyNavigate(1)
      await firstRow.focus()

      const announce = board.reorderAnnounce()
      await expect(announce).toBeAttached()

      await firstRow.press('Control+Shift+ArrowDown')

      await expect(announce).toContainText(/Moved to position \d+ of 3/, { timeout: 3_000 })

      await expect(async () => {
        const order = await board.storyOrder('Ready')
        expect(order[0]).not.toBe(1)
      }).toPass({ timeout: 3_000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('first row Ctrl+Shift+Up is a no-op (boundary)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const firstRow = board.storyNavigate(1)
      await firstRow.focus()

      await firstRow.press('Control+Shift+ArrowUp')

      const order = await board.storyOrder('Ready')
      expect(order).toEqual([1, 2, 3])

      await expect(board.reorderAnnounce()).toHaveText('')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('last row Ctrl+Shift+Down is a no-op (boundary)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const lastRow = board.storyNavigate(3)
      await lastRow.focus()

      await lastRow.press('Control+Shift+ArrowDown')

      const order = await board.storyOrder('Ready')
      expect(order).toEqual([1, 2, 3])

      await expect(board.reorderAnnounce()).toHaveText('')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('row aria-label names the reorder affordance', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMembers(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const button = board.storyNavigate(1)
      const ariaLabel = await button.getAttribute('aria-label')
      expect(ariaLabel).toMatch(/position 1 of 3/i)
      expect(ariaLabel).toMatch(/Ctrl\+Shift/i)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
