import { test, expect } from '@playwright/test'
import { join } from 'path'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import {
  setupCurrentIterationViaRpc,
  addStoryToIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const createStory = async (rpc: RpcFn, title: string): Promise<void> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job: `Fixture story for desk-check #390: ${title}.`,
      observation_falsifier: 'reject if the blocked indicator does not render as expected',
      acceptance_test_path: 'applications/electron/tests/desk-check-390.spec.ts'
    },
    'pm'
  )
  expect(resp.status).toBe(200)
}

const driveToDelivered = async (
  rpc: RpcFn,
  id: number,
  acceptor: 'owner' | 'team'
): Promise<void> => {
  const startResp = await rpc('start_story', {
    id,
    pickup_paraphrase: 'Driving the chain so the delivered badge is reachable.',
    falsifier_reframing: 'reject if start_story does not move story to in-progress'
  })
  expect(startResp.status).toBe(200)
  const deliverResp = await rpc('deliver_story', {
    id,
    verified_observation: 'Drove deliver so designer can pass desk-check next.'
  })
  expect(deliverResp.status).toBe(200)
  await seedTeamVerdicts(rpc, id, ['designer'])
  const passResp = await rpc('pass_desk_check', { id, acceptor, would_file_bug: false }, 'designer')
  expect(passResp.status).toBe(200)
}

const addLabel = async (rpc: RpcFn, id: number, label: string): Promise<void> => {
  const resp = await rpc('label_story', { id, label }, 'pm')
  expect(resp.status).toBe(200)
}

const seedFixture = async (result: MockGhLaunchResult): Promise<void> => {
  const { rpc } = result
  await createStory(rpc, '[Feature] Blocker story (drill-in target)')
  await createStory(rpc, '[Feature] Delivered blocked with blocker reference')
  await createStory(rpc, '[Feature] Delivered blocked without blocker reference')
  await createStory(rpc, '[Feature] Delivered unblocked awaiting owner')

  const iterationId = await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck390')
  await addStoryToIterationViaRpc(rpc, 2, iterationId, 2)
  await addStoryToIterationViaRpc(rpc, 3, iterationId, 3)
  await addStoryToIterationViaRpc(rpc, 4, iterationId, 4)

  await driveToDelivered(rpc, 1, 'owner')
  await driveToDelivered(rpc, 2, 'owner')
  await addLabel(rpc, 2, 'blocked')
  await addLabel(rpc, 2, 'blocked-by:1')
  await driveToDelivered(rpc, 3, 'owner')
  await addLabel(rpc, 3, 'blocked')
  await driveToDelivered(rpc, 4, 'owner')
}

const openDelivered = async (board: BoardPage): Promise<void> => {
  await board.openColumn('Acceptance')
}

test.describe('desk-check #390 blocked visual indicator', () => {
  test('blocked label renders distinct badge + tooltip with blocker reference', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await openDelivered(board)

      const blocked = board.storyStateBadge(2)
      await expect(blocked).toHaveClass(/story-badge--blocked\b/)
      await expect(blocked).toContainText('blocked')
      await expect(blocked).toHaveAttribute('title', /waiting on #1/)
      await expect(blocked).toHaveAttribute('aria-label', /waiting on #1/)
      await expect(blocked).toHaveAttribute('aria-label', /open blocker/)

      await expect(board.storyRow(2)).toHaveClass(/board-story-row--blocked/)

      await win.screenshot({
        path: join(EVIDENCE_DIR, '99-blocked-badge-with-reference.png'),
        fullPage: true
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('blocked without blocked-by falls back to no-reference text — still visible', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await openDelivered(board)

      const blocked = board.storyStateBadge(3)
      await expect(blocked).toHaveClass(/story-badge--blocked\b/)
      await expect(blocked).toContainText('blocked')
      await expect(blocked).toHaveAttribute('title', /no blocker specified/)

      await expect(board.storyRow(3)).toHaveClass(/board-story-row--blocked/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('unblocked delivered row has no blocked class or blocked badge', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await openDelivered(board)

      const baseline = board.storyStateBadge(4)
      await expect(baseline).toHaveClass(/story-badge--accepted\b/)
      await expect(baseline).toContainText('awaiting owner')

      await expect(board.storyRow(4)).not.toHaveClass(/board-story-row--blocked/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('click on blocked badge opens the blocker drill-in', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await openDelivered(board)

      await board.storyStateBadge(2).click()

      const drill = drillIn.panel()
      await expect(drill).toBeVisible()
      await expect(drill).toContainText('#1')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
