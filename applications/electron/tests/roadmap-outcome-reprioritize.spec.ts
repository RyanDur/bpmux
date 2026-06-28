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

test.describe('#14 — the owner reprioritizes outcomes within a band from the GUI', () => {
  test('dragging an outcome over a peer re-orders the band over the WS, live, and survives reload', async () => {
    test.setTimeout(90_000)
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()

      const leads = 'Agents own the substrate'
      const trails = 'The team runs its own infra'
      const leadsNum = await numberFromMessage(
        await rpc('create_outcome', { title: leads, horizon: 'now', body: '**Benefit:** a' }, 'pm')
      )
      const trailsNum = await numberFromMessage(
        await rpc('create_outcome', { title: trails, horizon: 'now', body: '**Benefit:** b' }, 'pm')
      )

      await win.reload()
      await expect(board.region()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await expect(roadmap.bandOutcomeNames('Now')).toHaveText([leads, trails])

      await roadmap.dragOutcome(leadsNum, trailsNum)

      await expect(roadmap.bandOutcomeNames('Now')).toHaveText([trails, leads])

      await win.reload()
      await expect(board.region()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await expect(roadmap.bandOutcomeNames('Now')).toHaveText([trails, leads])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
