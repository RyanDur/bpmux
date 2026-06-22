import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

test.describe('#1596 close/kill (S5) — an EXITED terminal is recoverable (restart) or removable (close)', () => {
  test('an exited terminal offers restart (re-spawns a live terminal) and close (removes the dead pane)', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await pt.waitReady()

      await pt.focusActive()
      await pt.runUntilItemLabel(1, 'exit', 'exited')

      await expect(pt.restartButton()).toBeVisible()
      await expect(pt.paneCloseButton()).toBeVisible()

      await pt.restartButton().click()
      await pt.waitReady()

      await pt.focusActive()
      await pt.runUntilItemLabel(1, 'exit', 'exited')

      await pt.paneCloseButton().click()
      await expect(pt.items()).toHaveCount(0, { timeout: 10000 })
      await expect(pt.output()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
