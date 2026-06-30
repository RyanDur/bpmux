import { test, expect } from '@playwright/test'
import { launchTwoElectronsSharedBackend } from './helpers/launch'
import { BoardPage } from './pages/boardPage'

test.describe('#1300 Scenario 2 — two bpmux instances see the same iteration-close outcome', () => {
  test('close_iteration in instance A closes the container in instance B via WS state-delta; members are NOT promoted (promote-on-close retired #1556)', async () => {
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
              id: 9401,
              title: 'Iteration Cetus',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              id: 9411,
              title: 'cetus-alpha',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Cetus.100']
            },
            {
              id: 9412,
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
      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9401)).toBeVisible({ timeout: 20_000 })
      await expect(board2.iterationGrip(9401)).toBeVisible({ timeout: 20_000 })

      const stateOf = async (): Promise<{
        iterationClosed: boolean
        memberStates: Record<number, string>
      }> => {
        const body = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        }).then(
          (r) =>
            r.json() as Promise<{
              stories: { id: number; state: string }[]
            }>
        )
        const iterations = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        }).then(
          (r) =>
            r.json() as Promise<{
              iterations: { id: number; complete: boolean }[]
            }>
        )
        return {
          iterationClosed: iterations.iterations.find((s) => s.id === 9401)?.complete === true,
          memberStates: Object.fromEntries(
            body.stories.filter((s) => s.id === 9411 || s.id === 9412).map((s) => [s.id, s.state])
          )
        }
      }

      const closeRes = await fetch(`${apiBase}/api/close_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({ iteration_id: 9401 })
      })
      expect(closeRes.ok).toBe(true)

      await board1.openClosedIterationsSection()
      await board2.openClosedIterationsSection()
      await expect(
        board1.closedIterationsSection().getByText('Iteration Cetus', { exact: true })
      ).toBeVisible({ timeout: 15_000 })
      await expect(
        board2.closedIterationsSection().getByText('Iteration Cetus', { exact: true })
      ).toBeVisible({ timeout: 15_000 })

      const finalState = await stateOf()
      expect(finalState.iterationClosed).toBe(true)
      expect(finalState.memberStates[9411]).toBe('icebox')
      expect(finalState.memberStates[9412]).toBe('icebox')
    } finally {
      await cleanup()
    }
  })
})
