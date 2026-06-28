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

test.describe('#14 — the owner moves an outcome between horizon bands from the GUI', () => {
  test('picking a new band dispatches over the WS, the card moves live, and it survives reload', async () => {
    test.setTimeout(90_000)
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()

      const title = 'The substrate stays honest'
      await numberFromMessage(
        await rpc('create_outcome', { title, horizon: 'now', body: '**Benefit:** trust' }, 'pm')
      )
      await win.reload()
      await expect(board.region()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await expect(roadmap.outcomeInBand('Now', title)).toBeVisible()

      await roadmap.moveToBand(title, 'Next')

      await expect(roadmap.outcomeInBand('Next', title)).toBeVisible()
      await expect(roadmap.band('Now')).toHaveCount(0)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await expect(roadmap.outcomeInBand('Next', title)).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
