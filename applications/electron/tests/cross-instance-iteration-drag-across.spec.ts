import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('#1339 Scenario 2 — drag-across iteration cross-instance: member move visible in both', () => {
  test('move a member from iteration A to iteration B in one instance propagates to the other', async () => {
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
              id: 9601,
              title: 'Iteration Volans',
              story_type: 'iteration',
              initial_state: 'icebox',
              body: '<!-- bpmux-relationships: {"stories":[9611,9620]} -->'
            },
            {
              id: 9602,
              title: 'Iteration Crater',
              story_type: 'iteration',
              initial_state: 'icebox',
              body: '<!-- bpmux-relationships: {"stories":[9621]} -->'
            },
            { id: 9611, title: 'volans-member', story_type: 'feature', initial_state: 'icebox' },
            { id: 9620, title: 'volans-anchor', story_type: 'feature', initial_state: 'icebox' },
            { id: 9621, title: 'crater-anchor', story_type: 'feature', initial_state: 'icebox' }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9601)).toBeVisible({ timeout: 15_000 })
      await expect(board2.iterationGrip(9602)).toBeVisible({ timeout: 15_000 })

      const memberIn = async (board: BoardPage, iteration: number): Promise<boolean> =>
        (await board.memberOrder(iteration)).includes(9611)

      expect(await memberIn(board1, 9601)).toBe(true)
      expect(await memberIn(board1, 9602)).toBe(false)
      expect(await memberIn(board2, 9601)).toBe(true)
      expect(await memberIn(board2, 9602)).toBe(false)

      const moveRes = await fetch(`${apiBase}/api/rpc/add_story_to_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
          'X-BPMUX-Agent': 'pm'
        },
        body: JSON.stringify({ iteration_id: 9602, story_id: 9611, position: 1 })
      })
      expect(moveRes.ok, await moveRes.text()).toBe(true)

      await expect
        .poll(() => memberIn(board1, 9602), { timeout: 15_000, intervals: [200] })
        .toBe(true)
      await expect
        .poll(() => memberIn(board2, 9602), { timeout: 15_000, intervals: [200] })
        .toBe(true)

      expect(await memberIn(board1, 9601)).toBe(false)
      expect(await memberIn(board2, 9601)).toBe(false)
    } finally {
      await cleanup()
    }
  })
})
