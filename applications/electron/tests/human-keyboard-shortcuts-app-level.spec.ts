import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// #1127 sub-slice 3: app-level keyboard shortcuts. Pure renderer
// behavior — no story-state setup needed; mock-GH defaults are
// sufficient (empty issues list).

test.describe('human-side E2E #476 — direct-action — app-level keyboard shortcuts', () => {
  test('Ctrl+P toggles board panel visibility', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = win.locator('[data-test="board-region"]')
      await expect(board).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const titlebar = win.locator('[data-test="titlebar"]')
      await titlebar.click()

      await win.keyboard.press('Control+p')
      await expect(board).toBeHidden()

      await win.keyboard.press('Control+p')
      await expect(board).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Ctrl+Shift+M toggles metrics overlay visibility', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="app-root"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      // Metrics renders as a native <dialog>, always mounted, with
      // visibility gated by the `open` attribute. Assert visibility
      // (not count) per the post-#497 dialog-mount shape — `toHaveCount(0)`
      // was the pre-#497 conditional-mount idiom.
      const overlay = win.locator('[data-test="metrics-overlay"]')
      await expect(overlay).toBeHidden()

      await win.keyboard.press('Control+Shift+M')
      await expect(overlay).toBeVisible()

      await win.keyboard.press('Control+Shift+M')
      await expect(overlay).toBeHidden()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
