import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('human-side E2E #476 — cross-process — typed message reaches other agent', () => {
  test('human-typed broadcast appears in /api/messages?as=<peer-role> visibility-filter', async () => {
    const { app, win, mockGh, backendPort } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const textarea = win.locator('[data-test="chat-input-textarea"]')
      await expect(textarea).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const broadcastMarker = 'CROSS_PROCESS_BROADCAST_MARKER_BPF19'
      await textarea.fill(broadcastMarker)
      await textarea.press('Enter')
      await expect(textarea).toHaveValue('')

      const list = win.locator('[data-test="message-list"]')
      await expect(list).toContainText(broadcastMarker, { timeout: 10000 })

      for (const peer of ['dev', 'pm', 'designer']) {
        const resp = await fetch(
          `http://127.0.0.1:${backendPort}/api/messages?as=${peer}&limit=200`
        )
        expect(resp.status).toBe(200)
        const messages = (await resp.json()) as Array<{ text: string }>
        expect(
          messages.some((m) => m.text === broadcastMarker),
          `peer "${peer}" should observe the broadcast in their visibility-filtered log`
        ).toBe(true)
      }

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
