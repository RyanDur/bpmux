import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck367', '2026-04-22T16:00:00Z'),
    {
      number: 367,
      title: 'Timeline audit-trail',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Timeline story body.',
      ...inMockCurrentIteration(1)
    }
  ]
}

const MOCK_COMMENTS: unknown[] = []

// Fixture timeline with a canonical lifecycle AND one rework cycle:
//  filed → started → delivered → desk-check-fail → delivered →
//  accepted → closed. The rework cycle should group events 3..5
//  (fail + redeliver + accept) under a single reworkGroup.
const MOCK_TIMELINE = [
  {
    event: 'filed',
    actor: 'ryandurling',
    ts: '2026-04-22T16:00:00Z'
  },
  {
    event: 'labeled',
    actor: 'dev-bot',
    ts: '2026-04-22T16:05:00Z',
    label: 'in progress'
  },
  {
    event: 'labeled',
    actor: 'dev-bot',
    ts: '2026-04-22T16:30:00Z',
    label: 'delivered'
  },
  {
    event: 'labeled',
    actor: 'designer-bot',
    ts: '2026-04-22T16:45:00Z',
    label: 'desk-check-fail'
  },
  {
    event: 'labeled',
    actor: 'dev-bot',
    ts: '2026-04-22T17:00:00Z',
    label: 'delivered'
  },
  {
    event: 'labeled',
    actor: 'ryandurling',
    ts: '2026-04-22T17:10:00Z',
    label: 'accepted'
  },
  {
    event: 'closed',
    actor: 'ryandurling',
    ts: '2026-04-22T17:11:00Z'
  },
  {
    event: 'commented',
    actor: 'ryandurling',
    ts: '2026-04-22T17:12:00Z',
    body: 'Great work team — this shipped clean.'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-367-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-367-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/367/comments', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_COMMENTS)
    })
  })
  await win.route('**/api/stories/367/timeline', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_TIMELINE)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #367 timeline audit-trail', () => {
  test('drill-in exposes a timeline section with major gates by default', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const storyButton = win.locator('[data-test="board-tabpanel-now-content"] [data-test="board-story-367"] .board-story-navigate')
    await storyButton.click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

    const timeline = win.locator('[data-test="drill-in-timeline"]')
    await expect(timeline).toBeVisible()

    // Expand the details.
    await timeline.locator('summary').click()
    await expect(win.locator('[data-test="drill-in-timeline-list"]')).toBeVisible()

    // Default filter: every MAJOR_GATE event should be present.
    // (commented is NOT a major gate; should be hidden until "show
    // all" is toggled.)
    const items = win.locator('[data-test="drill-in-timeline-item"]')
    const count = await items.count()
    console.log(`#367 default timeline item count: ${count}`)

    // Expected default gates: filed, started (labeled in progress),
    // delivered x2, desk-check-fail, accepted, closed = 7 items.
    expect(count).toBe(7)

    // Commented event body should NOT appear in the default view.
    await expect(
      win.locator('[data-test="drill-in-timeline-item"][data-gate="commented"]')
    ).toHaveCount(0)

    // Rework group: the 2nd delivered + accepted items carry the
    // rework-group attribute because they followed the desk-check-fail.
    const reworkItems = win.locator('[data-test="drill-in-timeline-item"][data-rework-group]')
    const reworkCount = await reworkItems.count()
    console.log(`#367 rework-grouped items: ${reworkCount}`)
    expect(reworkCount).toBeGreaterThanOrEqual(3) // fail + redeliver + accept

    await timeline.screenshot({
      path: join(EVIDENCE_DIR, '80-timeline-default-major-gates.png')
    })

    await app.close()
  })

  test('show-all toggle reveals non-gate events', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const storyButton = win.locator('[data-test="board-tabpanel-now-content"] [data-test="board-story-367"] .board-story-navigate')
    await storyButton.click()
    const timeline = win.locator('[data-test="drill-in-timeline"]')
    await timeline.locator('summary').click()

    await win.locator('[data-test="drill-in-timeline-toggle"]').click()

    // After show-all, the commented event should render.
    const commentedItem = win.locator('[data-test="drill-in-timeline-item"][data-gate="commented"]')
    await expect(commentedItem).toHaveCount(1)
    await expect(commentedItem).toContainText('Great work team')

    await app.close()
  })
})
