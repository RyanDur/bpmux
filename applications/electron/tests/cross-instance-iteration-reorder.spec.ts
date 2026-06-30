import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('#1339 Scenario 1 — two bpmux instances see the same iteration order after one reorders', () => {
  test('drag-reorder in instance A propagates to instance B via WS state-delta', async () => {
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
              id: 9301,
              title: 'Iteration Reticulum',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              id: 9311,
              title: 'reticulum-alpha',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['parent-iteration-#9301.100']
            },
            {
              id: 9312,
              title: 'reticulum-bravo',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['parent-iteration-#9301.200']
            },
            {
              id: 9313,
              title: 'reticulum-charlie',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['parent-iteration-#9301.300']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await win1.reload()
      await win2.reload()
      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9301)).toBeVisible({ timeout: 20_000 })
      await expect(board2.iterationGrip(9301)).toBeVisible({ timeout: 20_000 })

      const initial1 = await board1.memberOrder(9301)
      const initial2 = await board2.memberOrder(9301)
      expect(initial1).toEqual([9311, 9312, 9313])
      expect(initial2).toEqual([9311, 9312, 9313])

      const mutateRes = await fetch(`${apiBase}/api/stories/9313/labels`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          add: ['parent-iteration-#9301.50'],
          remove: ['parent-iteration-#9301.300']
        })
      })
      expect(mutateRes.ok).toBe(true)

      await expect
        .poll(() => board1.memberOrder(9301), { timeout: 15_000, intervals: [200] })
        .toEqual([9313, 9311, 9312])
      await expect
        .poll(() => board2.memberOrder(9301), { timeout: 15_000, intervals: [200] })
        .toEqual([9313, 9311, 9312])
    } finally {
      await cleanup()
    }
  })
})
