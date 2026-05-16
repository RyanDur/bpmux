import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'

test.describe('#1339 Scenario 2 — drag-across iteration cross-instance: member retag visible in both', () => {
  test('retag member from iteration A to iteration B in instance A propagates to instance B', async () => {
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
              number: 9601,
              title: 'Iteration Volans',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              number: 9602,
              title: 'Iteration Crater',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              number: 9611,
              title: 'volans-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Volans.100']
            },
            {
              number: 9620,
              title: 'volans-anchor',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Volans.200']
            },
            {
              number: 9621,
              title: 'crater-anchor',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Crater.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      const volansSelector = '[data-test="board-iteration-9601"]'
      const craterSelector = '[data-test="board-iteration-9602"]'
      await expect(win1.locator(volansSelector)).toBeVisible({ timeout: 15_000 })
      await expect(win2.locator(craterSelector)).toBeVisible({ timeout: 15_000 })

      const memberInContainer = async (
        page: typeof win1,
        containerSelector: string
      ): Promise<boolean> => {
        return await page.evaluate((sel) => {
          const container = document.querySelector(sel)
          return container?.querySelector('[data-iteration-member="9611"]') !== null
        }, containerSelector)
      }

      expect(await memberInContainer(win1, volansSelector)).toBe(true)
      expect(await memberInContainer(win1, craterSelector)).toBe(false)
      expect(await memberInContainer(win2, volansSelector)).toBe(true)
      expect(await memberInContainer(win2, craterSelector)).toBe(false)

      const mutateRes = await fetch(`${apiBase}/api/stories/9611/labels`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          add: ['iteration-Crater.100'],
          remove: ['iteration-Volans.100']
        })
      })
      expect(mutateRes.ok).toBe(true)

      await expect
        .poll(async () => await memberInContainer(win1, craterSelector), {
          timeout: 15_000,
          intervals: [200]
        })
        .toBe(true)
      await expect
        .poll(async () => await memberInContainer(win2, craterSelector), {
          timeout: 15_000,
          intervals: [200]
        })
        .toBe(true)

      expect(await memberInContainer(win1, volansSelector)).toBe(false)
      expect(await memberInContainer(win2, volansSelector)).toBe(false)
    } finally {
      await cleanup()
    }
  })
})
