import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron, launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { seedCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const NO_ITERATION_BOARD = {
  fetched_at: '2026-06-16T00:00:00Z',
  stories: [
    {
      id: 820,
      title: 'an iceboxed feature',
      state: 'icebox',
      gh_state: 'open',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-06-16T00:00:00Z',
      body: ''
    }
  ]
}

test.describe('#1684 — no active iteration → the board defaults to the roadmap-view', () => {
  test('cold render with no current iteration: the Now view shows the roadmap (not a dead board) and the Board tab is centered', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-1684-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-1684-udd-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    try {
      await win.route('**/api/board', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(NO_ITERATION_BOARD)
        })
      })
      await win.reload()
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()

      await expect(roadmap.nowRoadmap()).toBeVisible()

      await expect(roadmap.headerIteration()).toHaveCount(0)
      await expect
        .poll(() => roadmap.boardTabLabel().evaluate((el) => getComputedStyle(el).justifyContent))
        .toBe('center')
    } finally {
      await app.close()
    }
  })

  test('LIVE on iteration-close: the Now view switches to the roadmap and the Board tab re-centers with no reload (#1720 freshness)', async () => {
    const { app, win, mockGh, rpc, backendPort, authToken } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })

      await waitForBoardLoaded(win)
      const iterationId = await seedCurrentIterationViaRpc(rpc, 'Crux1684')
      await win.reload()
      await waitForBoardLoaded(win)

      const roadmap = new RoadmapPage(win)
      await expect(roadmap.headerIteration()).toHaveText('Crux1684')
      await expect(roadmap.nowRoadmap()).toHaveCount(0)

      const closeRes = await fetch(`http://127.0.0.1:${backendPort}/api/close_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({ iteration_id: iterationId })
      })
      expect(closeRes.ok).toBe(true)

      await expect(roadmap.nowRoadmap()).toBeVisible({ timeout: 15_000 })
      await expect(roadmap.headerIteration()).toHaveCount(0)
      await expect
        .poll(() => roadmap.boardTabLabel().evaluate((el) => getComputedStyle(el).justifyContent))
        .toBe('center')

      expect(consoleErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
