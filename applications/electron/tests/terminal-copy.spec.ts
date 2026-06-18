import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1598 copy (S7) — Cmd-C copies the terminal selection; Ctrl-C still interrupts', () => {
  test('select output + Cmd-C copies it to the OS clipboard, and Ctrl-C still raises SIGINT (not shadowed)', async () => {
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
      await activeScreen.click()
      await win.keyboard.type('echo COPYMARKER_8s7x')
      await win.keyboard.press('Enter')
      await expect(activeScreen).toContainText('COPYMARKER_8s7x', { timeout: 15000 })

      const box = await activeScreen.boundingBox()
      if (!box) throw new Error('terminal screen has no bounding box')
      await win.mouse.move(box.x + 14, box.y + 14)
      await win.mouse.down()
      await win.mouse.move(box.x + box.width - 14, box.y + box.height - 14, { steps: 16 })
      await win.mouse.up()

      await win.keyboard.press('Meta+c')

      await expect
        .poll(() => app.evaluate(({ clipboard }) => clipboard.readText()), {
          timeout: 10000,
          intervals: [300, 500, 800, 1200]
        })
        .toContain('COPYMARKER_8s7x')

      await activeScreen.click()
      await win.keyboard.type('sleep 30')
      await win.keyboard.press('Enter')
      await win.waitForTimeout(800)
      await win.keyboard.press('Control+c')
      await win.keyboard.type('echo AFTER:$((3*3))')
      await win.keyboard.press('Enter')
      await expect(activeScreen).toContainText('AFTER:9', { timeout: 10000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
