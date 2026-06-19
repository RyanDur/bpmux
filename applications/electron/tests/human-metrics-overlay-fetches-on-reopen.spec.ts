import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { MetricsPage } from './pages/metricsPage'

// #497 slice 2 regression-guard: the native <dialog> migration moved
// Metrics from conditionally-mounted (each open = fresh component =
// fresh useEffect = fresh fetch) to always-mounted. The fetch
// useEffect must include `open` in its deps so reopens trigger a
// fresh GET /api/metrics — otherwise stale-from-Chat-mount-time
// data persists across closes/reopens.

test.describe('human-side E2E #497 — metrics dialog fetches on reopen', () => {
  test('reopening metrics overlay fires a fresh /api/metrics fetch', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      const metricsRequests: string[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))
      win.on('request', (req) => {
        if (req.url().includes('/api/metrics')) {
          metricsRequests.push(`${req.method()} ${req.url()}`)
        }
      })

      const metrics = new MetricsPage(win)
      await expect(win.getByRole('main')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0
      metricsRequests.length = 0

      await win.keyboard.press('Control+Shift+M')
      const overlay = metrics.dialog()
      await expect.poll(() => overlay.evaluate((el: HTMLDialogElement) => el.open)).toBe(true)
      await expect.poll(() => metricsRequests.length).toBeGreaterThanOrEqual(1)
      const firstFetchCount = metricsRequests.length

      await metrics.closeButton().click()
      await expect.poll(() => overlay.evaluate((el: HTMLDialogElement) => el.open)).toBe(false)

      await win.keyboard.press('Control+Shift+M')
      await expect.poll(() => overlay.evaluate((el: HTMLDialogElement) => el.open)).toBe(true)
      await expect
        .poll(() => metricsRequests.length, { timeout: 5000 })
        .toBeGreaterThan(firstFetchCount)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
