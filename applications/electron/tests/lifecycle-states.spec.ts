import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1581 terminal lifecycle states — a failed/exited terminal is legible, never blank', () => {
  test('a real shell that EXITS shows "exited" (server PtyExit), not a blank working pane', async () => {
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
      await win.keyboard.type('exit')
      await win.keyboard.press('Enter')

      await expect(item).toHaveAttribute('data-state', 'exited', { timeout: 15000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('a failed spawn shows "spawn-failed" loudly, never a blank pane', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/nonexistent/bpf1581-bogus-shell' }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const item = win.locator('[data-test="sidebar-terminal-item"]')
      await expect(item).toHaveAttribute('data-state', 'spawn-failed', { timeout: 15000 })

      const statePane = win.locator('[data-test^="terminal-state-"]')
      await expect(statePane).toContainText('spawn-failed')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
