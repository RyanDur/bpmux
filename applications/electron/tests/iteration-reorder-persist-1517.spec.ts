import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { issueJson } from './helpers/mock_gh'

test.describe('#1517 iteration-row reorder persists across a real-GH reload round-trip', () => {
  test('iterations seeded WITHOUT an order label (as create_iteration makes them) reorder by drag and the order survives drain-to-GitHub + ingest + reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: {
        BPMUX_POLL_INTERVAL_SECS: '2',
        BPMUX_DRAIN_INTERVAL_SECS: '1'
      }
    })
    try {
      mockGh.setIssues([
        issueJson(9701, 'Iteration Norma', ['iteration'], ''),
        issueJson(9702, 'Iteration Octans', ['iteration'], '')
      ])

      await expect(win.locator('[data-test="app-root"]')).toBeVisible()
      await win.locator('[data-test="board-tab-plan"]').click()

      const orderIn = async (): Promise<number[]> =>
        win.evaluate(() => {
          const list = document.querySelector('.board-iterations-list')
          if (!list) return []
          return Array.from(list.querySelectorAll('[data-test^="board-iteration-"]'))
            .map((el) => el.getAttribute('data-test'))
            .filter((t): t is string => t !== null && /^board-iteration-\d+$/.test(t))
            .map((t) => Number(t.replace('board-iteration-', '')))
        })

      await expect.poll(orderIn, { timeout: 20_000, intervals: [250] }).toEqual([9701, 9702])

      const grip = win.locator(
        '.board-iteration-item:has([data-test="board-iteration-9702"]) [data-test="iteration-grip"]'
      )
      const target = win.locator('[data-test="board-iteration-9701"]')
      await expect(grip).toBeVisible()
      await grip.dragTo(target)

      await expect.poll(orderIn, { timeout: 5_000, intervals: [100] }).toEqual([9702, 9701])

      const labelRequestCount = (): number =>
        mockGh.requestLog().filter((r) => /\/issues\/9702\/labels/.test(r.path)).length
      await expect.poll(labelRequestCount, { timeout: 15_000, intervals: [250] }).toBeGreaterThan(0)

      const orderLabelOn9702 = async (): Promise<string | undefined> => {
        const res = await fetch(`${mockGh.baseUrl}repos/acme/bpmux/issues`)
        const ghIssues = (await res.json()) as Array<{
          number: number
          labels: Array<{ name: string }>
        }>
        return ghIssues
          .find((i) => i.number === 9702)
          ?.labels.map((l) => l.name)
          .find((n) => n.startsWith('iteration-order.'))
      }
      await expect.poll(orderLabelOn9702, { timeout: 15_000, intervals: [250] }).toBeDefined()

      await new Promise((resolve) => setTimeout(resolve, 3_000))
      await win.reload()
      await expect(win.locator('[data-test="app-root"]')).toBeVisible()
      await win.locator('[data-test="board-tab-plan"]').click()

      await expect.poll(orderIn, { timeout: 20_000, intervals: [250] }).toEqual([9702, 9701])
    } finally {
      await app.close()
    }
  })
})
