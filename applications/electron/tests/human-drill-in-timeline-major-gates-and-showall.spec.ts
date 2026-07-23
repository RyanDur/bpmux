import { test, expect } from '@playwright/test'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'
import type { MockGhServer } from './helpers/mock_gh'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MEMBER_TITLE = 'Timeline audit-trail'

const labeledEvent = (label: string, created_at: string): Record<string, unknown> => ({
  event: 'labeled',
  id: null,
  node_id: null,
  url: null,
  actor: null,
  commit_id: null,
  commit_url: null,
  created_at,
  label: { name: label, color: 'ededed' }
})

const MOCK_GH_TIMELINE: Record<string, unknown>[] = [
  labeledEvent('in progress', '2026-04-22T16:05:00Z'),
  labeledEvent('delivered', '2026-04-22T16:30:00Z'),
  labeledEvent('desk-check-fail', '2026-04-22T16:45:00Z'),
  labeledEvent('delivered', '2026-04-22T17:00:00Z'),
  labeledEvent('accepted', '2026-04-22T17:10:00Z'),
  {
    event: 'closed',
    id: null,
    node_id: null,
    url: null,
    actor: null,
    commit_id: null,
    commit_url: null,
    created_at: '2026-04-22T17:11:00Z'
  },
  {
    event: 'commented',
    id: null,
    node_id: null,
    url: null,
    actor: null,
    commit_id: null,
    commit_url: null,
    created_at: '2026-04-22T17:12:00Z',
    body: 'Great work team — this shipped clean.'
  }
]

const seedFixture = async (rpc: RpcFn, mockGh: MockGhServer): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${MEMBER_TITLE}`,
      user_job: 'When a human opens a story, the drill-in shows a timeline of major gates.',
      observation_falsifier: 'reject if the timeline omits a major gate or mis-groups rework',
      acceptance_test_path:
        'applications/electron/tests/human-drill-in-timeline-major-gates-and-showall.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck367')
  mockGh.on({
    method: 'GET',
    match: /\/repos\/acme\/bpmux\/issues\/1\/timeline/,
    bodyJson: MOCK_GH_TIMELINE
  })
}

test.describe('desk-check #367 timeline audit-trail', () => {
  test('drill-in exposes a timeline section with major gates by default', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
      await expect(drillIn.panel()).toBeVisible()

      const timeline = drillIn.timelineSection()
      await expect(timeline).toBeVisible()

      await drillIn.timelineSummary().click()
      await expect(drillIn.timelineList()).toBeVisible()

      const count = await drillIn.timelineItems().count()
      console.log(`#367 default timeline item count: ${count}`)

      expect(count).toBe(7)

      await expect(drillIn.timelineItems().filter({ hasText: 'Commented' })).toHaveCount(0)

      const reworkCount = await drillIn.reworkItems().count()
      console.log(`#367 rework-grouped items: ${reworkCount}`)
      expect(reworkCount).toBeGreaterThanOrEqual(3)

      await timeline.screenshot({
        path: join(EVIDENCE_DIR, '80-timeline-default-major-gates.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('show-all toggle reveals non-gate events', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
      await drillIn.timelineSummary().click()

      await drillIn.timelineToggle().click()

      const commentedItem = drillIn.timelineItems().filter({ hasText: 'Commented' })
      await expect(commentedItem).toHaveCount(1)
      await expect(commentedItem).toContainText('Great work team')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
