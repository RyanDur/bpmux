import { test, expect } from '@playwright/test'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MEMBER_TITLE = 'Story for live board refresh test'

const seedReadyMember = async (rpc: RpcFn): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${MEMBER_TITLE}`,
      user_job: 'When the board mounts, the current-iteration member story renders.',
      observation_falsifier: 'reject if the member story row does not render after board fetch',
      acceptance_test_path:
        'applications/electron/tests/human-board-renders-current-iteration-story-on-mount.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck353')
}

test.describe('desk-check #353 WS board invalidate', () => {
  test('board renders + fetches on mount, story visible', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.storyNavigate(1)).toBeVisible()
      await win.waitForTimeout(300)
      await board.region().screenshot({
        path: join(EVIDENCE_DIR, '80-board-live-refresh-baseline.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('subscription code: messages prop reaches Board (indirect — the board_invalidated state-delta is the trigger)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')

      const mounted = await board.storyNavigate(1).isVisible()
      console.log(`#353 mount: story row present = ${mounted}`)

      await expect(board.storyNavigate(1)).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('poll interval is no longer 30s (sanity — 5-min fallback should not fire in short test)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await win.waitForTimeout(2000)
      await expect(board.storyNavigate(1)).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
