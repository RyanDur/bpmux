import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('terminal-restore resilience (#1751) — a transient /api/terminals failure is retried', () => {
  test('a first-attempt network failure is retried and does NOT surface a restore error', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    try {
      const restoreErrors: string[] = []
      win.on('console', (m) => {
        if (m.type() === 'error' && m.text().includes('terminal restore failed')) {
          restoreErrors.push(m.text())
        }
      })

      let calls = 0
      await win.route('**/api/terminals', async (route) => {
        calls += 1
        if (calls === 1) {
          await route.abort('failed')
          return
        }
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ terminals: [] })
        })
      })

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      await expect.poll(() => calls, { timeout: 8000 }).toBeGreaterThanOrEqual(2)
      expect(restoreErrors).toEqual([])
    } finally {
      await app.close()
    }
  })
})
