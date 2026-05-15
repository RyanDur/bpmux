import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'

test.describe('#1300 Scenario 2 — two bpmux instances see the same iteration-close outcome', () => {
  test('close_iteration in instance A promotes members + closes container in instance B via WS state-delta', async () => {
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
              number: 9401,
              title: 'Iteration Cetus',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              number: 9411,
              title: 'cetus-alpha',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Cetus.100']
            },
            {
              number: 9412,
              title: 'cetus-bravo',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Cetus.200']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await win1.reload()
      await win2.reload()

      const iterSelector = '[data-test="board-iteration-9401"]'
      await expect(win1.locator(iterSelector)).toBeVisible({ timeout: 20_000 })
      await expect(win2.locator(iterSelector)).toBeVisible({ timeout: 20_000 })

      const stateOf = async (): Promise<{
        iterationClosed: boolean
        memberStates: Record<number, string>
      }> => {
        const body = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        }).then((r) => r.json() as Promise<{
          stories: { number: number; state: string; gh_state?: string }[]
        }>)
        return {
          iterationClosed:
            body.stories.find((s) => s.number === 9401)?.gh_state === 'closed',
          memberStates: Object.fromEntries(
            body.stories
              .filter((s) => s.number === 9411 || s.number === 9412)
              .map((s) => [s.number, s.state])
          )
        }
      }

      const closeRes = await fetch(`${apiBase}/api/close_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({ iteration_number: 9401 })
      })
      expect(closeRes.ok).toBe(true)

      const renderedClosedAttr = async (page: typeof win1): Promise<string | null> => {
        return await page.evaluate(() => {
          const el = document.querySelector('[data-test="board-iteration-9401"]')
          return el ? (el as HTMLElement).getAttribute('data-closed') : null
        })
      }

      await expect
        .poll(async () => await renderedClosedAttr(win1), {
          timeout: 15_000,
          intervals: [200]
        })
        .toBe('true')
      await expect
        .poll(async () => await renderedClosedAttr(win2), {
          timeout: 15_000,
          intervals: [200]
        })
        .toBe('true')

      const finalState = await stateOf()
      expect(finalState.iterationClosed).toBe(true)
      expect(finalState.memberStates[9411]).toBe('ready')
      expect(finalState.memberStates[9412]).toBe('ready')
    } finally {
      await cleanup()
    }
  })
})
