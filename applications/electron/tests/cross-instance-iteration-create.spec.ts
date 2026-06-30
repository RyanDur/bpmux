import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('#1297 Scenario 3 — create iteration in one instance is visible in the other', () => {
  test('seed_stories iteration in instance A propagates to instance B via WS state-delta + BOARD_INVALIDATE', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)

      await expect(board1.iterationGrip(9501)).toHaveCount(0)
      await expect(board2.iterationGrip(9501)).toHaveCount(0)

      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9501,
              title: 'Iteration Pavo',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              id: 9502,
              title: 'pavo-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Pavo.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9501)).toBeVisible({ timeout: 15_000 })
      await expect(board2.iterationGrip(9501)).toBeVisible({ timeout: 15_000 })

      await expect(
        board1.iterationRow(9501).getByText('Iteration Pavo', { exact: true })
      ).toBeVisible()
      await expect(
        board2.iterationRow(9501).getByText('Iteration Pavo', { exact: true })
      ).toBeVisible()
    } finally {
      await cleanup()
    }
  })
})
