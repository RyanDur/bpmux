import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1466/#1595 real-$SHELL via ＋ — spawning a terminal from the terminals section runs a REAL shell whose executed output renders', () => {
  test('clicking ＋ spawns a real $SHELL terminal: a typed command executes and its output renders (not a printf double, not a blank pane)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const activeTerminal = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(activeTerminal).toBeVisible({ timeout: 10000 })
      await activeTerminal.click()

      await expect
        .poll(
          async () => {
            await win.keyboard.type('echo TERM=$TERM')
            await win.keyboard.press('Enter')
            return (await activeTerminal.textContent()) ?? ''
          },
          { timeout: 20000, intervals: [400, 700, 1000, 1500] }
        )
        .toContain('TERM=xterm-256color')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
