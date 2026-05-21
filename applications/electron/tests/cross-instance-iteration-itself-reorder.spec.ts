import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'

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
              number: 9701,
              title: 'Iteration Norma',
              story_type: 'iteration',
              initial_state: 'icebox',
              labels: ['iteration-order.100']
            },
            {
              number: 9702,
              title: 'Iteration Octans',
              story_type: 'iteration',
              initial_state: 'icebox',
              labels: ['iteration-order.200']
            },
            {
              number: 9710,
              title: 'norma-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Norma.100']
            },
            {
              number: 9711,
              title: 'octans-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Octans.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      const orderIn = async (page: typeof win1): Promise<number[]> => {
        return await page.evaluate(() => {
          const list = document.querySelector('.board-iterations-list')
          if (!list) return []
          return Array.from(list.querySelectorAll('[data-test^="board-iteration-"]'))
            .map((el) => el.getAttribute('data-test'))
            .filter((t): t is string => t !== null && /^board-iteration-\d+$/.test(t))
            .map((t) => Number(t.replace('board-iteration-', '')))
        })
      }

      await expect
        .poll(async () => await orderIn(win1), { timeout: 15_000, intervals: [200] })
        .toEqual([9701, 9702])
      await expect
        .poll(async () => await orderIn(win2), { timeout: 15_000, intervals: [200] })
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
        .poll(async () => await orderIn(win1), { timeout: 15_000, intervals: [200] })
        .toEqual([9702, 9701])
      await expect
        .poll(async () => await orderIn(win2), { timeout: 15_000, intervals: [200] })
        .toEqual([9702, 9701])
    } finally {
      await cleanup()
    }
  })
})
