import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('#1339 Scenario 3 — iteration-itself reorder cross-instance: container order visible in both', () => {
  test('iteration-order label mutation in instance A propagates container order in instance B', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9701,
              title: 'Iteration Norma',
              story_type: 'iteration',
              initial_state: 'icebox',
              labels: ['iteration-order.100']
            },
            {
              id: 9702,
              title: 'Iteration Octans',
              story_type: 'iteration',
              initial_state: 'icebox',
              labels: ['iteration-order.200']
            },
            {
              id: 9710,
              title: 'norma-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Norma.100']
            },
            {
              id: 9711,
              title: 'octans-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Octans.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect
        .poll(() => board1.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9701, 9702])
      await expect
        .poll(() => board2.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9701, 9702])

      const mutateRes = await fetch(`${apiBase}/api/stories/9702/labels`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          add: ['iteration-order.50'],
          remove: ['iteration-order.200']
        })
      })
      expect(mutateRes.ok).toBe(true)

      await expect
        .poll(() => board1.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9702, 9701])
      await expect
        .poll(() => board2.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9702, 9701])
    } finally {
      await cleanup()
    }
  })
})
