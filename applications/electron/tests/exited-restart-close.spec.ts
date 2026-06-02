import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

test.describe('#1596 close/kill (S5) — an EXITED terminal is recoverable (restart) or removable (close)', () => {
  test('an exited terminal offers restart (re-spawns a live terminal) and close (removes the dead pane)', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      const item = win.locator('[data-test="sidebar-terminal-item"]')

      const exitActive = async (): Promise<void> => {
        await win
          .locator('[data-test^="terminal-slot-"][data-active="true"] .terminal-screen')
          .click()
        await win.keyboard.type('exit')
        await win.keyboard.press('Enter')
      }

      await addTerminal.click()
      await expect(item).toHaveAttribute('data-state', 'ready', { timeout: 15000 })

      await exitActive()
      await expect(item).toHaveAttribute('data-state', 'exited', { timeout: 15000 })

      await expect(win.locator('[data-test^="terminal-restart-"]')).toBeVisible()
      await expect(win.locator('[data-test^="terminal-pane-close-"]')).toBeVisible()

      await win.locator('[data-test^="terminal-restart-"]').click()
      await expect(item).toHaveAttribute('data-state', 'ready', { timeout: 15000 })

      await exitActive()
      await expect(item).toHaveAttribute('data-state', 'exited', { timeout: 15000 })

      await win.locator('[data-test^="terminal-pane-close-"]').click()
      await expect(item).toHaveCount(0, { timeout: 10000 })
      await expect(win.locator('[data-test^="terminal-slot-"]')).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
