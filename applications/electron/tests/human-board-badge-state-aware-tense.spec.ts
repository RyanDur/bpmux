import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
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

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const createStory = async (rpc: RpcFn, title: string): Promise<void> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job: `Fixture story for desk-check #375: ${title}.`,
      observation_falsifier: 'reject if the state-aware badge label is wrong for this fixture row',
      acceptance_test_path: atpPath(__filename)
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

const driveToAccepted = async (
  rpc: RpcFn,
  id: number,
  acceptor: 'owner' | 'team'
): Promise<void> => {
  await driveToDelivered(rpc, id, acceptor)
  await seedTeamVerdicts(rpc, id, ['designer', 'dev'])
  const acceptRole = acceptor === 'owner' ? 'owner' : 'pm'
  const acceptResp = await rpc('accept_story', { id, would_file_bug: false }, acceptRole)
  expect(acceptResp.status).toBe(200)
}

const seedFixture = async (result: MockGhLaunchResult): Promise<void> => {
  const { rpc } = result
  await createStory(rpc, '[Feature] Delivered feature awaiting owner')
  await createStory(rpc, '[Chore] Delivered chore awaiting team')
  await createStory(rpc, '[Feature] Accepted feature')
  await createStory(rpc, '[Chore] Accepted chore')

  const iterationId = await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck375')
  await addStoryToIterationViaRpc(rpc, 2, iterationId, 2)

  await driveToDelivered(rpc, 1, 'owner')
  await driveToDelivered(rpc, 2, 'team')
  await driveToAccepted(rpc, 3, 'owner')
  await driveToAccepted(rpc, 4, 'team')
}

const GROUP = {
  acceptance: { tab: 'Now', label: 'Acceptance' },
  accepted: { tab: 'Done', label: 'Accepted' }
} as const

const openGroup = async (board: BoardPage, state: 'acceptance' | 'accepted'): Promise<void> => {
  const { tab, label } = GROUP[state]
  await board.openTab(tab)
  await board.openColumn(label)
}

test.describe('desk-check #375 state-aware badge labels', () => {
  test('delivered rows use awaiting-tense labels', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await openGroup(board, 'acceptance')

      const feat = board.storyStateBadge(1)
      await expect(feat).toHaveClass(/story-badge--accepted\b/)
      await expect(feat).toContainText('awaiting owner')

      const chore = board.storyStateBadge(2)
      await expect(chore).toHaveClass(/story-badge--team-accepted\b/)
      await expect(chore).toContainText('awaiting team')

      await win.screenshot({
        path: join(EVIDENCE_DIR, '99-badge-state-aware-delivered.png'),
        fullPage: true
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('accepted rows use accepted-tense labels', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await openGroup(board, 'accepted')

      const feat = board.storyStateBadge(3)
      await expect(feat).toHaveClass(/story-badge--accepted\b/)
      await expect(feat).toContainText('owner-accepted')

      const chore = board.storyStateBadge(4)
      await expect(chore).toHaveClass(/story-badge--team-accepted\b/)
      await expect(chore).toContainText('team-accepted')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
