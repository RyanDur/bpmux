import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1596 close/kill (S5) — confirm-before-close when a foreground process is running', () => {
  test('closing a terminal with a running foreground process prompts to confirm; cancel keeps it, confirm-anyway closes it', async () => {
    test.setTimeout(60_000)
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

      const activeScreen = win.locator(
        '[data-test^="terminal-slot-"][data-active="true"] .terminal-screen'
      )
      await activeScreen.click()
      await win.keyboard.type('echo OUT:$((6*7))')
      await win.keyboard.press('Enter')
      await expect(activeScreen).toContainText('OUT:42', { timeout: 15000 })

      await win.keyboard.type('sleep 30')
      await win.keyboard.press('Enter')
      await win.waitForTimeout(1000)

      const dialog = win.locator('[data-test="confirm-close-dialog"]')

      await win.locator('[data-test="sidebar-terminal-close"]').click()
      await expect(dialog).toBeVisible({ timeout: 5000 })

      await win.locator('[data-test="confirm-close-cancel"]').click()
      await expect(dialog).toHaveCount(0)
      await expect(item).toHaveCount(1)

      await win.locator('[data-test="sidebar-terminal-close"]').click()
      await expect(dialog).toBeVisible({ timeout: 5000 })
      await win.locator('[data-test="confirm-close-confirm"]').click()
      await expect(item).toHaveCount(0, { timeout: 10000 })
      await expect(win.locator('[data-test^="terminal-slot-"]')).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
