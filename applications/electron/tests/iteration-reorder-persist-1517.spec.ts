import { test, expect, type Locator } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { issueJson } from './helpers/mock_gh'
import { BoardPage } from './pages/boardPage'

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

      await expect(win.getByRole('main')).toBeVisible()
      await new BoardPage(win).openTab('Plan')

      const orderIn = async (): Promise<number[]> =>
        win.evaluate(() =>
          Array.from(document.querySelectorAll('[aria-label^="Reorder iteration #"]'))
            .map((el) => (el.getAttribute('aria-label') ?? '').match(/Reorder iteration #(\d+)/))
            .filter((m): m is RegExpMatchArray => m !== null)
            .map((m) => Number(m[1]))
        )

      await expect.poll(orderIn, { timeout: 20_000, intervals: [250] }).toEqual([9701, 9702])

      const board = new BoardPage(win)
      const grip = board.iterationGrip(9702)
      const target = board.iterationRow(9701)
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
      await expect(win.getByRole('main')).toBeVisible()
      await new BoardPage(win).openTab('Plan')

      await expect.poll(orderIn, { timeout: 20_000, intervals: [250] }).toEqual([9702, 9701])
    } finally {
      await app.close()
    }
  })

  test('repeated reorders keep EXACTLY ONE order-label per iteration — no stale-label accumulation that corrupts the order on reload', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: {
        BPMUX_POLL_INTERVAL_SECS: '2',
        BPMUX_DRAIN_INTERVAL_SECS: '5'
      }
    })
    try {
      mockGh.setIssues([
        issueJson(9801, 'Iteration Norma', ['iteration'], ''),
        issueJson(9802, 'Iteration Octans', ['iteration'], ''),
        issueJson(9803, 'Iteration Pyxis', ['iteration'], '')
      ])

      await expect(win.getByRole('main')).toBeVisible()
      await new BoardPage(win).openTab('Plan')

      const orderIn = async (): Promise<number[]> =>
        win.evaluate(() =>
          Array.from(document.querySelectorAll('[aria-label^="Reorder iteration #"]'))
            .map((el) => (el.getAttribute('aria-label') ?? '').match(/Reorder iteration #(\d+)/))
            .filter((m): m is RegExpMatchArray => m !== null)
            .map((m) => Number(m[1]))
        )

      const board = new BoardPage(win)
      const gripFor = (n: number): Locator => board.iterationGrip(n)
      const rowOf = (n: number): Locator => board.iterationRow(n)

      const orderLabelCounts = async (): Promise<Record<number, number>> => {
        const res = await fetch(`${mockGh.baseUrl}repos/acme/bpmux/issues`)
        const issues = (await res.json()) as Array<{
          number: number
          labels: Array<{ name: string }>
        }>
        const out: Record<number, number> = {}
        for (const issue of issues) {
          out[issue.number] = issue.labels.filter((l) =>
            l.name.startsWith('iteration-order.')
          ).length
        }
        return out
      }

      await expect.poll(orderIn, { timeout: 20_000, intervals: [250] }).toEqual([9801, 9802, 9803])

      await gripFor(9803).dragTo(rowOf(9801))
      await expect.poll(orderIn, { timeout: 5_000, intervals: [100] }).toEqual([9803, 9801, 9802])

      await gripFor(9803).dragTo(rowOf(9802))
      await expect.poll(orderIn, { timeout: 5_000, intervals: [100] }).toEqual([9801, 9802, 9803])

      await new Promise((resolve) => setTimeout(resolve, 9_000))

      await expect
        .poll(orderLabelCounts, { timeout: 15_000, intervals: [250] })
        .toEqual({ 9801: 1, 9802: 1, 9803: 1 })

      await win.reload()
      await expect(win.getByRole('main')).toBeVisible()
      await new BoardPage(win).openTab('Plan')
      await expect.poll(orderIn, { timeout: 20_000, intervals: [250] }).toEqual([9801, 9802, 9803])
    } finally {
      await app.close()
    }
  })
})
