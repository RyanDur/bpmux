import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1581 terminal lifecycle states — a failed/exited terminal is legible, never blank', () => {
  test('a real shell that EXITS shows "exited" (server PtyExit), not a blank working pane', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await term.run('exit')

      await expect(term.item(1)).toContainText('exited', { timeout: 15000 })
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
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      await expect(term.item(1)).toContainText('failed', { timeout: 15000 })

      await expect(win.getByText('spawn-failed')).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
