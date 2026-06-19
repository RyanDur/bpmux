import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'

// Deterministic board fixture — one story per state we actively
// exercise. `delivered` covers the Accept/Reject owner path; `ready`
// covers default-visible list rendering.
const MOCK_BOARD = {
  fetched_at: '2026-04-21T00:00:00Z',
  stories: [
    mockCurrentIteration('AppSpec', '2026-04-18T12:00:00Z'),
    {
      number: 111,
      title: 'Ready story under test',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-18T12:00:00Z',
      body: 'Test body for #111',
      ...inMockCurrentIteration(1)
    },
    {
      number: 222,
      title: 'Delivered story awaiting accept',
      state: 'delivered',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-04-19T12:00:00Z',
      body: 'Test body for #222',
      ...inMockCurrentIteration(2)
    }
  ]
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
  const { app, win } = await launchIsolatedWithWindow()
  try {
    await win.route('**/api/board', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(MOCK_BOARD)
      })
    })
    await win.reload()

    const board = new BoardPage(win)
    const drillIn = new DrillInPage(win)
    await expect(board.region()).toBeVisible()
    await expect(board.storyRow(111)).toBeVisible()
    await expect(board.storyRow(222)).toBeVisible()

    await board.openStory(222)
    await expect(drillIn.panel()).toBeVisible()

    await expect(drillIn.acceptButton()).toBeVisible()
    await expect(drillIn.rejectButton()).toBeVisible()
  } finally {
    await app.close()
  }
})
