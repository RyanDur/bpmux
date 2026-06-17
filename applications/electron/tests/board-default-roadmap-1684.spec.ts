import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron, launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { seedCurrentIterationViaRpc } from './helpers/iteration-fixture'

const NO_ITERATION_BOARD = {
  fetched_at: '2026-06-16T00:00:00Z',
  stories: [
    {
      number: 820,
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
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const nowContent = win.locator('[data-test="board-tabpanel-now-content"]')
      await expect(nowContent.locator('[data-test="roadmap"]')).toBeVisible()
      await expect(win.locator('[data-test="board-now-empty"]')).toHaveCount(0)

      const boardTab = win.locator('[data-test="board-roadmap-tab-board"]')
      await expect(win.locator('[data-test="board-header-iteration"]')).toHaveCount(0)
      await expect
        .poll(() => boardTab.evaluate((el) => getComputedStyle(el).justifyContent))
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
      const iterationNumber = await seedCurrentIterationViaRpc(rpc, 'Crux1684')
      await win.reload()
      await waitForBoardLoaded(win)

      const nowContent = win.locator('[data-test="board-tabpanel-now-content"]')
      await expect(win.locator('[data-test="board-header-iteration"]')).toHaveText('Crux1684')
      await expect(nowContent.locator('[data-test="roadmap"]')).toHaveCount(0)

      const closeRes = await fetch(`http://127.0.0.1:${backendPort}/api/close_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({ iteration_number: iterationNumber })
      })
      expect(closeRes.ok).toBe(true)

      await expect(nowContent.locator('[data-test="roadmap"]')).toBeVisible({ timeout: 15_000 })
      await expect(win.locator('[data-test="board-header-iteration"]')).toHaveCount(0)
      await expect
        .poll(() =>
          win
            .locator('[data-test="board-roadmap-tab-board"]')
            .evaluate((el) => getComputedStyle(el).justifyContent)
        )
        .toBe('center')

      expect(consoleErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
