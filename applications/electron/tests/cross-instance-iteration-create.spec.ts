import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'

test.describe('#1297 Scenario 3 — create iteration in one instance is visible in the other', () => {
  test('seed_stories iteration in instance A propagates to instance B via WS state-delta + BOARD_INVALIDATE', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const iterSelector = '[data-test="board-iteration-9501"]'

      await expect(win1.locator(iterSelector)).toHaveCount(0)
      await expect(win2.locator(iterSelector)).toHaveCount(0)

      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              number: 9501,
              title: 'Iteration Pavo',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              number: 9502,
              title: 'pavo-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Pavo.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await win1.locator('[data-test="board-tab-plan"]').click()
      await win2.locator('[data-test="board-tab-plan"]').click()

      await expect(win1.locator(iterSelector)).toBeVisible({ timeout: 15_000 })
      await expect(win2.locator(iterSelector)).toBeVisible({ timeout: 15_000 })

      const titleIn = async (page: typeof win1): Promise<string | null> => {
        return await page.evaluate(() => {
          const el = document.querySelector('[data-test="board-iteration-9501"]')
          return el?.textContent?.includes('Iteration Pavo') ? 'Iteration Pavo' : null
        })
      }
      expect(await titleIn(win1)).toBe('Iteration Pavo')
      expect(await titleIn(win2)).toBe('Iteration Pavo')
    } finally {
      await cleanup()
    }
  })
})
