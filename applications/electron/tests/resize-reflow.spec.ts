import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1594 resize-reflow — resizing the window reflows the terminal (live cols×rows in the header)', () => {
  test('resizing the window changes the header live cols×rows (not a fixed 80×24)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const header = term.headerStatus()
      await expect(header).toBeVisible({ timeout: 10000 })
      await expect(header).not.toContainText('0×0', { timeout: 10000 })
      const before = (await header.textContent()) ?? ''

      await win.setViewportSize({ width: 480, height: 360 })

      await expect
        .poll(async () => (await header.textContent()) ?? '', {
          timeout: 10000,
          intervals: [300, 600, 1000]
        })
        .not.toBe(before)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the xterm BUFFER tracks the real width, not a fixed 80: a ~46-col line is one row at a wide viewport and wraps to multiple rows when narrowed', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const activeTerminal = term.output()
      await expect(activeTerminal).toBeVisible({ timeout: 10000 })
      await activeTerminal.click()

      const markersShareARow = async (start: string, end: string): Promise<boolean> =>
        win.evaluate(
          ([s, e]) => {
            const rows = Array.from(
              document.querySelectorAll('[data-active="true"] .xterm-rows > div')
            )
            return rows.some((r) => {
              const t = r.textContent ?? ''
              return t.includes(s) && t.includes(e)
            })
          },
          [start, end]
        )

      const markersBothPresent = async (start: string, end: string): Promise<boolean> =>
        win.evaluate(
          ([s, e]) => {
            const text = Array.from(
              document.querySelectorAll('[data-active="true"] .xterm-rows > div')
            )
              .map((r) => r.textContent ?? '')
              .join('\n')
            return text.includes(s) && text.includes(e)
          },
          [start, end]
        )

      await win.setViewportSize({ width: 1400, height: 900 })
      await expect
        .poll(
          async () => {
            await win.keyboard.type(`echo WIDESTART${'Z'.repeat(30)}WIDEEND`)
            await win.keyboard.press('Enter')
            return markersShareARow('WIDESTART', 'WIDEEND')
          },
          { timeout: 20000, intervals: [400, 700, 1000, 1500] }
        )
        .toBe(true)

      await win.setViewportSize({ width: 480, height: 600 })
      await expect
        .poll(
          async () => {
            await win.keyboard.type(`echo NARROWSTART${'Z'.repeat(30)}NARROWEND`)
            await win.keyboard.press('Enter')
            return markersBothPresent('NARROWSTART', 'NARROWEND')
          },
          { timeout: 20000, intervals: [400, 700, 1000, 1500] }
        )
        .toBe(true)
      expect(
        await markersShareARow('NARROWSTART', 'NARROWEND'),
        'a ~46-col line wraps at a narrow viewport — its start and end land on different xterm rows'
      ).toBe(false)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
