import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

test.describe('#468 board panel B2 — section-count badges', () => {
  test('badge visible in collapsed section header', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')

      await expect(board.columnToggle('Accepted')).toHaveAttribute('aria-expanded', 'false')

      await expect(board.columnCount('Accepted')).toBeVisible()
      await expect(board.columnCount('Accepted')).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('badge visible in expanded section header', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeExpanded')
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.columnToggle('Ready')).toHaveAttribute('aria-expanded', 'true')

      await expect(board.columnCount('Ready')).toBeVisible()
      await expect(board.columnCount('Ready')).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('badge format is (N) integer-in-parens', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeFormat')
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Done')
      await board.columnToggle('Accepted').click()
      await board.openTab('Plan')
      await board.openTab('Now')

      for (const label of ['Ready', 'In Progress', 'Acceptance']) {
        await expect(board.columnCount(label)).toHaveText(/^\(\d+\)$/)
      }
      await board.openTab('Done')
      await expect(board.columnCount('Accepted')).toHaveText(/^\(\d+\)$/)
      await board.openTab('Plan')
      await expect(board.columnCount('Icebox')).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('count matches the visible story rows in expanded section', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeCountMatch')
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.columnCount('Ready')).toHaveText(/^\(\d+\)$/)
      const countText = await board.columnCount('Ready').innerText()
      const n = Number.parseInt(countText.replace(/[()]/g, ''), 10)
      await expect(board.group('Ready').locator('[aria-label^="Open story #"]')).toHaveCount(n)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('empty (count=0) badge carries data-empty and renders in --fg-dim', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeEmpty')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const ready = board.columnCount('Ready')
      await expect(ready).toHaveText('(0)')
      await expect(ready).toHaveAttribute('data-empty', 'true')

      const dimRgb = await win.evaluate(() => {
        const probe = document.createElement('span')
        probe.style.color = 'var(--fg-dim)'
        document.body.appendChild(probe)
        const c = getComputedStyle(probe).color
        probe.remove()
        return c
      })
      const renderedColor = await ready.evaluate((el) => getComputedStyle(el).color)
      expect(renderedColor).toBe(dimRgb)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('clicking the count badge toggles the section (Coyle entire-area-clickable)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeClickToggle')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const summary = board.columnToggle('Ready')
      const count = board.columnCount('Ready')

      await expect(summary).toHaveAttribute('aria-expanded', 'true')
      await count.click()
      await expect(summary).toHaveAttribute('aria-expanded', 'false')
      await count.click()
      await expect(summary).toHaveAttribute('aria-expanded', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('count badge sits between chevron and label inside the summary', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeOrder')
      await win.reload()
      await expect(board.region()).toBeVisible()

      const summary = board.columnToggle('Ready')
      await expect(summary).toBeVisible()
      const order = await summary.evaluate((el) => Array.from(el.children).map((c) => c.className))
      expect(order[0]).toContain('board-group-chevron')
      expect(order[1]).toContain('board-group-count')
      expect(order[2]).toContain('board-group-label')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('no console errors on count render and toggle cycle', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeNoErrors')
      await win.reload()
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const count = board.columnCount('Ready')
      await count.click()
      await count.click()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
