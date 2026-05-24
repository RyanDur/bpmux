import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'

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
              number: 9301,
              title: 'Iteration Reticulum',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              number: 9311,
              title: 'reticulum-alpha',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['parent-iteration-#9301.100']
            },
            {
              number: 9312,
              title: 'reticulum-bravo',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['parent-iteration-#9301.200']
            },
            {
              number: 9313,
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
      await win1.locator('[data-test="board-tab-plan"]').click()
      await win2.locator('[data-test="board-tab-plan"]').click()

      const iter1Selector = '[data-test="board-iteration-9301"]'
      await expect(win1.locator(iter1Selector)).toBeVisible({ timeout: 20_000 })
      await expect(win2.locator(iter1Selector)).toBeVisible({ timeout: 20_000 })

      await win1.locator(iter1Selector).evaluate((el) => {
        ;(el as HTMLDetailsElement).open = true
      })
      await win2.locator(iter1Selector).evaluate((el) => {
        ;(el as HTMLDetailsElement).open = true
      })

      const memberOrderIn = async (page: typeof win1): Promise<number[]> => {
        return await page.evaluate(() => {
          const container = document.querySelector('[data-test="board-iteration-9301"]')
          if (!container) return []
          const members = Array.from(container.querySelectorAll('[data-iteration-member]'))
          return members.map((el) =>
            Number((el as HTMLElement).getAttribute('data-iteration-member'))
          )
        })
      }

      const initial1 = await memberOrderIn(win1)
      const initial2 = await memberOrderIn(win2)
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
        .poll(async () => await memberOrderIn(win1), { timeout: 15_000, intervals: [200] })
        .toEqual([9313, 9311, 9312])
      await expect
        .poll(async () => await memberOrderIn(win2), { timeout: 15_000, intervals: [200] })
        .toEqual([9313, 9311, 9312])
    } finally {
      await cleanup()
    }
  })
})
