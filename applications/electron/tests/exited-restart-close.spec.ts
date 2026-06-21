import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { runCommandUntilOutput } from './helpers/terminal'
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
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })

      const exitActive = async (): Promise<void> => {
        await term.output().click()
        await runCommandUntilOutput(win, 'exit', term.item(1), 'exited')
      }

      await term.addButton().click()
      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await exitActive()

      await expect(term.restartButton()).toBeVisible()
      await expect(term.paneCloseButton()).toBeVisible()

      await term.restartButton().click()
      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await exitActive()

      await term.paneCloseButton().click()
      await expect(term.items()).toHaveCount(0, { timeout: 10000 })
      await expect(term.output()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
