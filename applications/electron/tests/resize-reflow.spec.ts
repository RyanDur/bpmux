import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1594 resize-reflow — resizing the window reflows the terminal (live cols×rows in the header)', () => {
  test('resizing the window changes the header live cols×rows (not a fixed 80×24)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const header = win.locator('[data-test="terminal-header"]')
      await expect(header).toBeVisible({ timeout: 10000 })
      await expect(header).not.toContainText('0×0', { timeout: 10000 })
      const before = (await header.textContent()) ?? ''

      await win.setViewportSize({ width: 480, height: 360 })

      await expect
        .poll(async () => (await header.textContent()) ?? '', {
          timeout: 10000,
          intervals: [300, 600, 1000]
        })
        .not.toBe(before)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
