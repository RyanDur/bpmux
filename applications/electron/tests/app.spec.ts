import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'

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
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
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

    await expect(win.locator('[data-test="board-region"]')).toBeVisible()
    const nowPanel = win.locator('[data-test="board-tabpanel-now-content"]')
    await expect(nowPanel.locator('[data-test="board-story-111"]')).toBeVisible()
    await expect(nowPanel.locator('[data-test="board-story-222"]')).toBeVisible()

    await nowPanel.locator('[data-test="board-story-222"] .board-story-navigate').click({
      position: { x: 4, y: 4 }
    })
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

    await expect(win.locator('[data-test="drill-in-accept"]')).toBeVisible()
    await expect(win.locator('[data-test="drill-in-reject"]')).toBeVisible()
  } finally {
    await app.close()
  }
})
