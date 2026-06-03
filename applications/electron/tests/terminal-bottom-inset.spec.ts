import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1611 terminal bottom inset — the bottom row is not clipped past the screen edge', () => {
  test('with the terminal filled past the bottom, the xterm rows fit inside the screen (no row cut by overflow:hidden)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()
      const active = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(active).toBeVisible({ timeout: 10000 })
      await active.click()

      await win.keyboard.type(
        'for i in $(seq 1 90); do echo "ROW_$i ===bottombottombottom==="; done'
      )
      await win.keyboard.press('Enter')
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
