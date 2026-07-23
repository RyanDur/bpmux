import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const createStory = async (rpc: RpcFn, title: string): Promise<number> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job: `Board-header fixture: ${title}.`,
      observation_falsifier: 'reject if the iteration member does not render',
      acceptance_test_path: atpPath(__filename)
    },
    'pm'
  )
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const number = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(number).toBeGreaterThan(0)
  return number
}

test.describe('#1510 board-header iteration-context', () => {
  test('iteration-name leads the BOARD-header (left, primary) with BOARD + sync-glyph trailing; anchor-box retired', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)

      const memberNumber = await createStory(rpc, '[Feature] Member of Alpha')
      await setupCurrentIterationViaRpc(rpc, memberNumber, 'Alpha')

      await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe('dark')

      const header = roadmap.boardTabLabel()
      const iteration = roadmap.headerIteration()
      const title = header.locator('.board-title')
      const glyph = header.getByRole('status')

      await expect(iteration).toBeVisible()
      await expect(iteration).toHaveText('Alpha')
      await expect(title).toHaveText('Board')
      await expect(glyph).toBeVisible()

      const iterBox = await iteration.boundingBox()
      const titleBox = await title.boundingBox()
      const glyphBox = await glyph.boundingBox()
      expect(iterBox).not.toBeNull()
      expect(titleBox).not.toBeNull()
      expect(glyphBox).not.toBeNull()
      expect(
        iterBox!.x,
        'iteration-name leads on the LEFT, before the BOARD panel-marker'
      ).toBeLessThan(titleBox!.x)
      expect(
        titleBox!.x,
        'BOARD label sits left of the trailing sync-glyph (BOARD ● grouped on the right)'
      ).toBeLessThan(glyphBox!.x)

      const iterColor = await iteration.evaluate((el) => getComputedStyle(el).color)
      const primary = await win.evaluate(() => {
        const probe = document.createElement('span')
        probe.style.color = 'var(--fg-primary)'
        document.body.appendChild(probe)
        const c = getComputedStyle(probe).color
        probe.remove()
        return c
      })
      expect(iterColor, 'iteration-name carries --fg-primary (the hero weight)').toBe(primary)

      await board.openTab('Now')
      await expect(board.nowContent()).toBeVisible()

      const shotPath = join(tmpdir(), 'bpmux-1510-board-header.png')
      await win.screenshot({ path: shotPath, fullPage: false })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
