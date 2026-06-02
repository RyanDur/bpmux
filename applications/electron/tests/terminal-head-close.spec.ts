import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1596 close/kill (S5) — the term-head close (✕ in the header)', () => {
  test('the ✕ in the terminal header tears down a ready terminal (the second close affordance)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const item = win.locator('[data-test="sidebar-terminal-item"]')
      await expect(item).toHaveAttribute('data-state', 'ready', { timeout: 15000 })

      const headerClose = win.locator('[data-test="terminal-header-close"]')
      await expect(headerClose).toBeVisible()
      await win.waitForTimeout(2000)
      await headerClose.click()

      await expect(item).toHaveCount(0, { timeout: 10000 })
      await expect(win.locator('[data-test^="terminal-slot-"]')).toHaveCount(0)
      await expect(win.locator('[data-test="message-list"]')).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
