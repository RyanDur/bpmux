import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedCurrentIterationViaRpc } from './helpers/iteration-fixture'

test.describe('#468 board panel B2 — section-count badges', () => {
  test('badge visible in collapsed section header', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await win.locator('[data-test="board-tab-done"]').click()

      const acceptedSummary = win.locator('[data-test="board-group-accepted"]')
      await expect(acceptedSummary).toHaveAttribute('aria-expanded', 'false')

      const acceptedCount = win.locator('[data-test="board-group-accepted-count"]')
      await expect(acceptedCount).toBeVisible()
      await expect(acceptedCount).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('badge visible in expanded section header', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeExpanded')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const readySummary = win.locator('[data-test="board-group-ready"]')
      await expect(readySummary).toHaveAttribute('aria-expanded', 'true')

      const readyCount = win.locator('[data-test="board-group-ready-count"]')
      await expect(readyCount).toBeVisible()
      await expect(readyCount).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('badge format is (N) integer-in-parens', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeFormat')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await win.locator('[data-test="board-tab-done"]').click()
      await win.locator('[data-test="board-group-accepted"]').click()
      await win.locator('[data-test="board-tab-plan"]').click()
      await win.locator('[data-test="board-tab-now"]').click()

      const states = ['ready', 'in-progress', 'delivered']
      for (const state of states) {
        const count = win.locator(
          `[data-test="board-tabpanel-now-content"] [data-test="board-group-${state}-count"]`
        )
        await expect(count).toHaveText(/^\(\d+\)$/)
      }
      await win.locator('[data-test="board-tab-done"]').click()
      await expect(
        win.locator('[data-test="board-tabpanel-done-content"] [data-test="board-group-accepted-count"]')
      ).toHaveText(/^\(\d+\)$/)
      await win.locator('[data-test="board-tab-plan"]').click()
      await expect(
        win.locator('[data-test="board-tabpanel-plan-content"] [data-test="board-group-icebox-count"]')
      ).toHaveText(/^\(\d+\)$/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('count matches the visible story rows in expanded section', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeCountMatch')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const readyContent = win.locator('#board-group-ready-content')
      const rows = readyContent.locator('[data-test^="board-story-"]')
      const visible = await rows.count()

      const countText = await win.locator('[data-test="board-group-ready-count"]').innerText()
      const n = Number.parseInt(countText.replace(/[()]/g, ''), 10)
      expect(n).toBe(visible)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('empty (count=0) badge carries data-empty and renders in --fg-dim', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeEmpty')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const ready = win.locator('[data-test="board-group-ready-count"]')
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
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeClickToggle')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const summary = win.locator('[data-test="board-group-ready"]')
      const count = win.locator('[data-test="board-group-ready-count"]')

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
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeOrder')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const order = await win.evaluate(() => {
        const summary = document.querySelector<HTMLElement>('[data-test="board-group-ready"]')
        if (!summary) return [] as string[]
        return Array.from(summary.children).map((el) => el.className)
      })
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

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await seedCurrentIterationViaRpc(rpc, 'BadgeNoErrors')
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const count = win.locator('[data-test="board-group-ready-count"]')
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
