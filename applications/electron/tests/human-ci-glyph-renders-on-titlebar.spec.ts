import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TitlebarPage } from './pages/titlebarPage'

test.describe('human-side E2E #476 — read-class — CI glyph', () => {
  test('CI glyph renders in titlebar with a conclusion attribute', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const titlebar = new TitlebarPage(win)
      await expect(titlebar.connectionStatus()).toBeVisible()

      const glyph = titlebar.ciGlyph()
      await expect(glyph).toBeVisible()
      const conclusion = await glyph.getAttribute('data-conclusion')
      expect(conclusion).not.toBeNull()
      expect(conclusion?.length ?? 0).toBeGreaterThan(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
