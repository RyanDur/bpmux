import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const MOCK_BOARD = {
  fetched_at: '2026-06-15T00:00:00Z',
  stories: [
    {
      number: 100,
      title: 'You see what is coming, and whether it is ready',
      state: 'icebox',
      gh_state: 'open',
      labels: ['outcome'],
      assignees: [],
      created_at: '2026-06-15T00:00:00Z',
      body: '**Benefit:** you steer the next iterations before convening\n**Confidence:** high',
      horizon: 'now',
      outcome_readiness: { iteration_count: 2, gap_count: 2, all_ready: false }
    },
    {
      number: 10,
      title: 'Crux',
      state: 'in-progress',
      gh_state: 'open',
      labels: ['iteration'],
      assignees: [],
      created_at: '2026-06-15T00:00:00Z',
      body: 'goal',
      outcome_membership: { outcome_number: 100, position: 1 },
      readiness: { drafted: true, stories_ordered: true, review_closed: false, dev_ok: false }
    },
    {
      number: 20,
      title: 'Lyra',
      state: 'in-progress',
      gh_state: 'open',
      labels: ['iteration'],
      assignees: [],
      created_at: '2026-06-15T00:00:00Z',
      body: '',
      outcome_membership: { outcome_number: 100, position: 2 },
      readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false }
    }
  ]
}

const beforeContent = (win, selector: string): Promise<string> =>
  win.evaluate((sel) => {
    const el = document.querySelector(sel)
    return el === null ? '' : getComputedStyle(el, '::before').content
  }, selector)

test.describe('#1737 roadmap outcome drill-in — focus an outcome → its iterations, in order', () => {
  test('focusing an outcome reveals the iterations building it (in order) with convene-checks + the CONVENE? banner', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-1737-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-1737-udd-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    const consoleErrors: string[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    try {
      await win.route('**/api/board', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(MOCK_BOARD)
        })
      })
      await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
      await win.reload()
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe('dark')

      await roadmap.selectView('Roadmap')
      await expect(roadmap.panel()).toBeVisible()

      const outcomeTitle = 'You see what is coming, and whether it is ready'
      await expect(roadmap.outcome(outcomeTitle)).toBeVisible()
      await expect(roadmap.drill()).toHaveCount(0)

      await roadmap.outcome(outcomeTitle).click()

      await expect(roadmap.drill()).toBeVisible()

      const names = await roadmap.iterationOrder().allTextContents()
      expect(names, 'the iterations building the outcome render IN ORDER').toEqual(['Crux', 'Lyra'])

      const convene = roadmap.convene()
      await expect(convene).toContainText('CONVENE?')
      await expect(convene).toContainText('Crux')
      await expect(convene).toContainText('designer-review')

      await expect(roadmap.checks('Crux')).not.toHaveCount(0)
      await expect(roadmap.checks('Lyra')).toHaveCount(0)
      await expect(roadmap.iterationRow('Lyra')).toHaveAttribute('data-state', 'not-started')

      await expect
        .poll(() => beforeContent(win, '.roadmap-drill-check[data-state="met"]'))
        .toContain('✓')
      await expect
        .poll(() => beforeContent(win, '.roadmap-drill-check[data-state="gap"]'))
        .toContain('✗')

      expect(consoleErrors, 'no console errors while drilling an outcome').toEqual([])

      const shotPath = join(tmpdir(), 'bpmux-1737-drill.png')
      await win.screenshot({ path: shotPath, fullPage: false })
    } finally {
      await app.close()
    }
  })
})
