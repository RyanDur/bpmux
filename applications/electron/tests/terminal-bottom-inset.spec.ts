import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1611 terminal bottom inset — the bottom row is not clipped past the screen edge', () => {
  test('with the terminal filled past the bottom, the xterm rows fit inside the screen (no row cut by overflow:hidden)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()
      const active = term.output()
      await expect(active).toBeVisible({ timeout: 10000 })

      await term.run('for i in $(seq 1 90); do echo "ROW_$i ===bottombottombottom==="; done')
      await win.waitForTimeout(2500)

      const fit = await win.evaluate(() => {
        const screen = document.querySelector('.terminal-screen')
        const rows = document.querySelector('.xterm-rows')
        const lastRow = rows?.lastElementChild
        const sr = screen?.getBoundingClientRect()
        const rr = rows?.getBoundingClientRect()
        const lr = lastRow?.getBoundingClientRect()
        return {
          rowsOverflowPastScreen: sr && rr ? rr.bottom - sr.bottom : 999,
          lastRowClearance: sr && lr ? sr.bottom - lr.bottom : -999
        }
      })

      expect(fit.rowsOverflowPastScreen).toBeLessThanOrEqual(0)
      expect(fit.lastRowClearance).toBeGreaterThanOrEqual(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
