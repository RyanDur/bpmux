import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { ChatPage } from './pages/chatPage'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1596 close/kill (S5) — the term-head close (✕ in the header)', () => {
  test('the ✕ in the terminal header tears down a ready terminal (the second close affordance)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      const chat = new ChatPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await expect(term.headerClose()).toBeVisible()
      await win.waitForTimeout(2000)
      await term.headerClose().click()

      await expect(term.items()).toHaveCount(0, { timeout: 10000 })
      await expect(term.output()).toHaveCount(0)
      await expect(chat.messageLog()).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
