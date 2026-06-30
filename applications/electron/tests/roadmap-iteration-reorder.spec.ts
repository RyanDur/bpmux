import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const numberFromMessage = async (resp: Response): Promise<number> => {
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const n = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(n).toBeGreaterThan(0)
  return n
}

test.describe('#14 — the owner reorders an outcome’s iterations from the drill-in', () => {
  test('dragging a building iteration re-sequences it over the WS, live, and it survives reload', async () => {
    test.setTimeout(90_000)
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()

      const title = 'Agents reason from the substrate'
      const outcome = await numberFromMessage(
        await rpc('create_outcome', { title, horizon: 'now', body: '**Benefit:** x' }, 'pm')
      )
      const crux = await numberFromMessage(await rpc('create_iteration', { name: 'Crux' }, 'pm'))
      const lyra = await numberFromMessage(await rpc('create_iteration', { name: 'Lyra' }, 'pm'))
      expect(
        (
          await rpc(
            'map_iteration_to_outcome',
            { outcome_id: outcome, iteration_id: crux, position: 1 },
            'pm'
          )
        ).status
      ).toBe(200)
      expect(
        (
          await rpc(
            'map_iteration_to_outcome',
            { outcome_id: outcome, iteration_id: lyra, position: 2 },
            'pm'
          )
        ).status
      ).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await roadmap.focusOutcome(title)
      await expect(roadmap.drillIterationNames()).toHaveText(['Iteration Crux', 'Iteration Lyra'])

      await roadmap.dragIteration(crux, lyra)

      await expect(roadmap.drillIterationNames()).toHaveText(['Iteration Lyra', 'Iteration Crux'])

      await win.reload()
      await expect(board.region()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await roadmap.focusOutcome(title)
      await expect(roadmap.drillIterationNames()).toHaveText(['Iteration Lyra', 'Iteration Crux'])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
