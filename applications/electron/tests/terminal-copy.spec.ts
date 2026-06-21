import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { runCommandUntilOutput } from './helpers/terminal'
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
      await runCommandUntilOutput(win, 'echo COPYMARKER_8s7x', activeScreen, 'COPYMARKER_8s7x')

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
      await runCommandUntilOutput(
        win,
        'echo SLEEPSTARTED_8s7x && sleep 30',
        activeScreen,
        'SLEEPSTARTED_8s7x'
      )
      await expect
        .poll(
          async () => {
            await win.keyboard.press('Control+c')
            await win.keyboard.type('echo AFTER:$((3*3))')
            await win.keyboard.press('Enter')
            return (await activeScreen.textContent()) ?? ''
          },
          { timeout: 20000, intervals: [500, 1000, 1500] }
        )
        .toContain('AFTER:9')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
