import { launchWithMockGh } from './helpers/launch'
import { MetricsPage } from './pages/metricsPage'
import { expect, test } from '@playwright/test'

test.describe('human-side E2E #476 — read-class — metrics overlay content', () => {
  test('opening metrics overlay renders the Velocity + DORA header + metric tiles', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const metrics = new MetricsPage(win)
      await expect(win.getByRole('main')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.keyboard.press('Control+Shift+M')

      const overlay = metrics.overlay()
      await expect(overlay).toBeVisible()
      await expect(metrics.heading()).toHaveText('Velocity + DORA')

      for (const label of [
        'Delivered / day',
        'Delivered in window',
        'Median pickup → deliver',
        'Median deliver → accept',
        'Rework rate'
      ]) {
        await expect(overlay).toContainText(label, { timeout: 10000 })
      }

      await metrics.closeButton().click()
      await expect
        .poll(() => metrics.dialog().evaluate((el: HTMLDialogElement) => el.open))
        .toBe(false)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

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
