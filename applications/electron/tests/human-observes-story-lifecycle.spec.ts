import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

const ATP = atpPath(__filename)

const watchErrors = (
  win: MockGhLaunchResult['win']
): { consoleErrors: string[]; pageErrors: Error[] } => {
  const consoleErrors: string[] = []
  const pageErrors: Error[] = []
  win.on('console', (m) => {
    if (m.type() === 'error') consoleErrors.push(m.text())
  })
  win.on('pageerror', (e) => pageErrors.push(e))
  return { consoleErrors, pageErrors }
}

const createStory = async (
  rpc: MockGhLaunchResult['rpc'],
  slug: string,
  falsifier: string
): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${slug}`,
      user_job: `When the team moves a story, the human needs the board to show ${slug} in its true column without manual refresh-thinking.`,
      observation_falsifier: falsifier,
      acceptance_test_path: ATP
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
}

const startStory = async (rpc: MockGhLaunchResult['rpc'], gate: string): Promise<void> => {
  const startResp = await rpc('start_story', {
    id: 1,
    pickup_paraphrase: `Picking up to verify ${gate} moves the story where the human expects.`,
    falsifier_reframing: `reject if ${gate} does not land the story in the column the human watches`
  })
  expect(startResp.status).toBe(200)
}

const deliverStory = async (rpc: MockGhLaunchResult['rpc'], gate: string): Promise<void> => {
  const deliverResp = await rpc('deliver_story', {
    id: 1,
    verified_observation: `Drove deliver via /api/rpc/deliver_story; queueing ${gate} as the gate under test.`
  })
  expect(deliverResp.status).toBe(200)
}

test.describe('the human watches a story move through the board — every gate lands it in the column they expect', () => {
  test('the happy path: start → In Progress, deliver → Desk Check, accept → Accepted, live then across reload', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const { consoleErrors, pageErrors } = watchErrors(win)
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      consoleErrors.length = 0
      pageErrors.length = 0

      await createStory(
        rpc,
        'lifecycle-happy-path',
        'reject if any gate lands the story in the wrong column'
      )
      await setupCurrentIterationViaRpc(rpc, 1, 'LifecycleHappyPath')

      await startStory(rpc, 'start_story')
      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect(board.storyInColumn('In Progress', 1)).toHaveCount(1)
      await expect(board.storyInColumn('In Progress', 1)).toHaveAttribute(
        'aria-label',
        /lifecycle-happy-path/
      )

      await deliverStory(rpc, 'accept_story')
      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect(board.storyInColumn('Desk Check', 1)).toHaveCount(1)

      await seedTeamVerdicts(rpc, 1, ['designer', 'dev'])
      const labelResp = await rpc(
        'label_story',
        { id: 1, label: 'awaiting team', rationale: 'acceptor=team for a team-verifiable accept' },
        'pm'
      )
      expect(labelResp.status).toBe(200)
      const acceptResp = await rpc('accept_story', { id: 1, would_file_bug: false }, 'pm')
      expect(acceptResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')
      await expect(board.storyInColumn('Accepted', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Accepted', 1)).toHaveAttribute(
        'aria-label',
        /lifecycle-happy-path/
      )

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the rework branch: fail_desk_check returns the story to In Progress and clears every later column', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const { consoleErrors, pageErrors } = watchErrors(win)
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      consoleErrors.length = 0
      pageErrors.length = 0

      await createStory(
        rpc,
        'lifecycle-rework',
        'reject if board does not show the story back in IN-PROGRESS after fail_desk_check'
      )
      await setupCurrentIterationViaRpc(rpc, 1, 'LifecycleRework')
      await startStory(rpc, 'fail_desk_check')
      await deliverStory(rpc, 'fail_desk_check')

      const failResp = await rpc(
        'fail_desk_check',
        { id: 1, reason: 'designer flagged accessibility regression on the touched surface' },
        'designer'
      )
      expect(failResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(
        board.group('In Progress').getByRole('button', { name: /lifecycle-rework/ })
      ).toBeVisible()
      for (const otherColumn of ['Ready', 'Desk Check', 'Acceptance']) {
        await expect(
          board.group(otherColumn).getByRole('button', { name: /Open story #1\b/ })
        ).toHaveCount(0)
      }

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the retire branch: close_story with a retire disposition lands in Retired, never Accepted', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const { consoleErrors, pageErrors } = watchErrors(win)
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      consoleErrors.length = 0
      pageErrors.length = 0

      await createStory(
        rpc,
        'lifecycle-retire',
        'reject if board does not show the story in RETIRED after close_story with a retire-class reason'
      )
      await setupCurrentIterationViaRpc(rpc, 1, 'LifecycleRetire')
      await startStory(rpc, 'close_story')

      const closeResp = await rpc(
        'close_story',
        {
          id: 1,
          reason: 'superseded by a sister filing — closing without accept-walk',
          disposition: 'retire'
        },
        'pm'
      )
      expect(closeResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')
      await expect(board.storyInColumn('Retired', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Accepted', 1)).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
