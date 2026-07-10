import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1596 close/kill (S5) — confirm-before-close when a foreground process is running', () => {
  test('closing a terminal with a running foreground process prompts to confirm; cancel keeps it, confirm-anyway closes it', async () => {
    test.setTimeout(60_000)
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

      const activeScreen = term.output()
      await term.run('echo OUT:$((6*7))')
      await expect(activeScreen).toContainText('OUT:42', { timeout: 15000 })

      await term.run('sleep 30')
      await win.waitForTimeout(1000)

      const dialog = term.confirmDialog()

      await term.closeButton(1).click()
      await expect(dialog).toBeVisible({ timeout: 5000 })

      await term.confirmCancel().click()
      await expect(dialog).toHaveCount(0)
      await expect(term.items()).toHaveCount(1)

      await term.closeButton(1).click()
      await expect(dialog).toBeVisible({ timeout: 5000 })
      await term.confirmAccept().click()
      await expect(term.items()).toHaveCount(0, { timeout: 10000 })
      await expect(term.output()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
