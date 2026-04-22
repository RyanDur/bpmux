import { test, expect, _electron as electron } from '@playwright/test'

// Deterministic board fixture — one story per state we actively
// exercise. `delivered` covers the Accept/Reject owner path; `ready`
// covers default-visible list rendering.
const MOCK_BOARD = {
  fetched_at: '2026-04-21T00:00:00Z',
  stories: [
    {
      number: 111,
      title: 'Ready story under test',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-18T12:00:00Z',
      body: 'Test body for #111'
    },
    {
      number: 222,
      title: 'Delivered story awaiting accept',
      state: 'delivered',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-04-19T12:00:00Z',
      body: 'Test body for #222'
    }
  ]
}

test('launches + connects + renders chat', async () => {
  const app = await electron.launch({ args: ['out/main/index.js'] })
  const win = await app.firstWindow()
  await expect(win.locator('[data-test="app-root"]')).toBeVisible()
  await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
  await app.close()
})

test('board renders stories, opens drill-in, shows Accept for delivered', async () => {
  const app = await electron.launch({ args: ['out/main/index.js'] })
  const win = await app.firstWindow()

  // Intercept the board fetch so the test is independent of whatever
  // the real backend has staged. We also intercept /api/stories/:n to
  // keep drill-in navigation clean; the accept endpoint is covered by
  // the mock-fallback path since we don't click it here (we only
  // assert visibility of the button).
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })

  await expect(win.locator('[data-test="board-region"]')).toBeVisible()
  // A story row renders.
  await expect(win.locator('[data-test="board-story-111"]')).toBeVisible()
  await expect(win.locator('[data-test="board-story-222"]')).toBeVisible()

  // Click into the delivered story.
  await win.locator('[data-test="board-story-222"] button').click()
  await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

  // Accept button visible for ryan (the hardcoded identity) on
  // delivered stories.
  await expect(win.locator('[data-test="drill-in-accept"]')).toBeVisible()
  await expect(win.locator('[data-test="drill-in-reject"]')).toBeVisible()

  await app.close()
})
