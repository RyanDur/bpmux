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

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const createStory = async (rpc: RpcFn, title: string): Promise<void> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job: `Fixture story for desk-check #376: ${title}.`,
      observation_falsifier: 'reject if the awaiting vs accepted badge color does not differ',
      acceptance_test_path: 'applications/electron/tests/desk-check-376.spec.ts'
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
  await createStory(rpc, '[Feature] delivered feature (awaiting owner)')
  await createStory(rpc, '[Feature] accepted feature (owner-accepted)')
  await createStory(rpc, '[Chore] delivered chore (awaiting team)')

  const iterationId = await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck376')
  await addStoryToIterationViaRpc(rpc, 3, iterationId, 3)

  await driveToDelivered(rpc, 1, 'owner')
  await driveToAccepted(rpc, 2, 'owner')
  await driveToDelivered(rpc, 3, 'team')
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

test.describe('desk-check #376 color-differentiate awaiting vs accepted', () => {
  test('data-state attribute lands on each badge', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await openGroup(board, 'acceptance')
      await expect(board.storyStateBadge(1)).toHaveAttribute('data-state', 'acceptance')
      await expect(board.storyStateBadge(3)).toHaveAttribute('data-state', 'acceptance')

      await openGroup(board, 'accepted')
      await expect(board.storyStateBadge(2)).toHaveAttribute('data-state', 'accepted')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('delivered and accepted badges render in different colors', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await openGroup(board, 'acceptance')
      const deliveredColor = await board
        .storyStateBadge(1)
        .evaluate((el: HTMLElement) => getComputedStyle(el).borderTopColor)

      await openGroup(board, 'accepted')
      const acceptedColor = await board
        .storyStateBadge(2)
        .evaluate((el: HTMLElement) => getComputedStyle(el).borderTopColor)

      console.log(`#376 delivered badge border: ${deliveredColor}`)
      console.log(`#376 accepted badge border: ${acceptedColor}`)
      expect(deliveredColor).not.toBe(acceptedColor)

      await win.screenshot({
        path: join(EVIDENCE_DIR, '100-badge-colors-differ.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
