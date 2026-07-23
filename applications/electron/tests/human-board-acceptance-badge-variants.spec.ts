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
import { DrillInPage } from './pages/drillInPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const createStory = async (rpc: RpcFn, title: string): Promise<void> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job: `Fixture story for desk-check #368: ${title}.`,
      observation_falsifier: 'reject if the badge variant does not render for this fixture row',
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

const seedFixture = async (result: MockGhLaunchResult): Promise<void> => {
  const { rpc } = result
  await createStory(rpc, '[Chore] Team-accepted chore')
  await createStory(rpc, '[Feature] Owner-accept feature')
  await createStory(rpc, '[Feature] Rework cycle story')
  await createStory(rpc, '[Feature] Plain ready — no badge')

  const iterationId = await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck368')
  await addStoryToIterationViaRpc(rpc, 2, iterationId, 2)
  await addStoryToIterationViaRpc(rpc, 3, iterationId, 3)
  await addStoryToIterationViaRpc(rpc, 4, iterationId, 4)

  await driveToDelivered(rpc, 1, 'team')
  await driveToDelivered(rpc, 2, 'owner')
  await driveToDelivered(rpc, 3, 'owner')
  const reworkResp = await rpc('label_story', { id: 3, label: 'desk-check-fail' }, 'pm')
  expect(reworkResp.status).toBe(200)

  const readyResp = await rpc('start_story', {
    id: 4,
    pickup_paraphrase: 'Leaving #4 in ready so it carries no acceptance badge.',
    falsifier_reframing: 'reject if a ready story renders an acceptance badge'
  })
  expect(readyResp.status).toBe(200)
}

test.describe('desk-check #368 team-accept badge', () => {
  test('team-accept / accept / rework badges render on expected stories', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openColumn('Acceptance')

      const chore = board.storyStateBadge(1)
      await expect(chore).toBeVisible()
      await expect(chore).toHaveClass(/story-badge--team-accepted\b/)
      await expect(chore).toContainText('awaiting team')

      const feature = board.storyStateBadge(2)
      await expect(feature).toHaveClass(/story-badge--accepted\b/)
      await expect(feature).toContainText('awaiting owner')

      const rework = board.storyStateBadge(3)
      await expect(rework).toHaveClass(/story-badge--rework\b/)
      await expect(rework).toContainText('rework')

      const plain = board.storyStateBadge(4)
      await expect(plain).toHaveCount(0)

      await win.screenshot({
        path: join(EVIDENCE_DIR, '90-badges-all-variants.png'),
        fullPage: true
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('clicking badge opens drill-in and auto-expands timeline', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(result)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openColumn('Acceptance')

      await board.storyStateBadge(1).click()

      await expect(drillIn.panel()).toBeVisible()

      const timeline = drillIn.timelineSection()
      await expect(timeline).toBeVisible()
      await expect(timeline).toHaveAttribute('open', '')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
