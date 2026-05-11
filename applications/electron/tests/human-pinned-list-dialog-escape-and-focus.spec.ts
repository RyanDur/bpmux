import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #496 — pinned-list native dialog', () => {
  test('open via show, escape closes via native dialog, no console errors', async () => {
    const seeded = {
      from: 'pm:claude-code',
      text: 'pinned-target — dialog migration regression',
      ts: '2026-04-22T16:00:00Z',
      message_type: 'chat'
    }
    const messageKey = `${seeded.ts}|${seeded.from}`
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [seeded],
      seedPins: {
        broadcast: [
          {
            message_key: messageKey,
            from: seeded.from,
            ts: seeded.ts,
            text: seeded.text,
            pinned_by: 'ryan',
            pinned_at: '2026-04-22T16:01:00Z'
          }
        ]
      },
      // #1290 Part B: GET /api/pins bypasses Storage trait (reads
      // pins.json directly from disk; REST-endpoint rewiring out of
      // scope per #1290). Opt back to FileStorage so seeded pins
      // reach the renderer's pin-count fetch.
      extraEnv: { BPMUX_STORAGE_BACKEND: 'file' }
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const pinButton = win.locator('[data-test="chat-pin-count"]')
      await expect(pinButton).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(pinButton).toBeEnabled()
      await pinButton.click()

      const overlay = win.locator('[data-test="pinned-list-overlay"]')
      await expect
        .poll(() => overlay.evaluate((el: HTMLDialogElement) => el.tagName))
        .toBe('DIALOG')
      await expect.poll(() => overlay.evaluate((el: HTMLDialogElement) => el.open)).toBe(true)
      await expect(overlay).toContainText(seeded.text)

      await overlay.press('Escape')
      await expect.poll(() => overlay.evaluate((el: HTMLDialogElement) => el.open)).toBe(false)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
