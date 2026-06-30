import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow, launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import {
  setupCurrentIterationViaRpc,
  addStoryToIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'

const createStory = async (rpc: RpcFn, title: string): Promise<number> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job: `App-spec fixture: ${title}.`,
      observation_falsifier: 'reject if the board does not render this fixture row',
      acceptance_test_path: 'applications/electron/tests/app.spec.ts'
    },
    'pm'
  )
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const number = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(number).toBeGreaterThan(0)
  return number
}

const driveToDelivered = async (rpc: RpcFn, id: number): Promise<void> => {
  const startResp = await rpc('start_story', {
    id,
    pickup_paraphrase: 'Driving to delivered so the Accept/Reject path is reachable.',
    falsifier_reframing: 'reject if start_story does not move story to in-progress'
  })
  expect(startResp.status).toBe(200)
  const deliverResp = await rpc('deliver_story', {
    id,
    verified_observation: 'Drove deliver so the owner-accept buttons render.'
  })
  expect(deliverResp.status).toBe(200)
  await seedTeamVerdicts(rpc, id, ['designer'])
  const passResp = await rpc(
    'pass_desk_check',
    { id, acceptor: 'owner', would_file_bug: false },
    'designer'
  )
  expect(passResp.status).toBe(200)
}

test('launches + connects + renders chat', async () => {
  const { app, win } = await launchIsolatedWithWindow()
  try {
    await expect(win.getByRole('main')).toBeVisible()
    await expect(new ChatPage(win).region()).toBeVisible()
  } finally {
    await app.close()
  }
})

test('board renders stories, opens drill-in, shows Accept for delivered', async () => {
  const { app, win, mockGh, rpc } = await launchWithMockGh()
  try {
    const board = new BoardPage(win)
    const drillIn = new DrillInPage(win)
    await expect(new ChatPage(win).region()).toBeVisible()

    const readyNumber = await createStory(rpc, '[Feature] Ready story under test')
    const deliveredNumber = await createStory(rpc, '[Feature] Delivered story awaiting accept')

    const iterationId = await setupCurrentIterationViaRpc(rpc, readyNumber, 'AppSpec')
    await addStoryToIterationViaRpc(rpc, deliveredNumber, iterationId, 2)
    await driveToDelivered(rpc, deliveredNumber)

    await win.reload()
    await expect(board.region()).toBeVisible()

    await board.openTab('Plan')
    await expect(board.storyRow(readyNumber)).toBeVisible()

    await board.openTab('Now')
    await board.openColumn('Acceptance')
    await expect(board.storyRow(deliveredNumber)).toBeVisible()

    await board.openStory(deliveredNumber)
    await expect(drillIn.panel()).toBeVisible()

    await expect(drillIn.acceptButton()).toBeVisible()
    await expect(drillIn.rejectButton()).toBeVisible()
  } finally {
    await app.close()
    await mockGh.close()
  }
})
