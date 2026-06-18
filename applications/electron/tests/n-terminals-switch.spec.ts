import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1595 N terminals — spawn two, switch, and neither is disposed (scrollback/running-state persist)', () => {
  test('＋ spawns distinct id-keyed terminals; switching toggles active and keeps BOTH mounted (not disposed)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })

      await term.addButton().click()
      const firstActive = term.activePane()
      await expect(firstActive).toBeVisible({ timeout: 10000 })
      const term1Key = (await firstActive.getAttribute('data-terminal-key')) ?? ''

      await term.addButton().click()
      await expect(term.items()).toHaveCount(2)

      const secondActive = term.activePane()
      await expect(secondActive).toBeVisible()
      const term2Key = (await secondActive.getAttribute('data-terminal-key')) ?? ''
      expect(term2Key).not.toBe(term1Key)

      const term1 = term.pane(term1Key)
      await expect(term1).toBeAttached()
      await expect(term1).toHaveAttribute('data-active', 'false')

      await term.item(1).click()
      await expect(term1).toHaveAttribute('data-active', 'true')

      const term2 = term.pane(term2Key)
      await expect(term2).toBeAttached()
      await expect(term2).toHaveAttribute('data-active', 'false')

      const label1 = (await term.item(1).textContent()) ?? ''
      const label2 = (await term.item(2).textContent()) ?? ''
      expect(label1).toContain('1 · ')
      expect(label2).toContain('2 · ')
      expect(label1).not.toBe(label2)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('a backgrounded terminal keeps streaming its live session: output emitted while inactive renders on switch-back, and it is still typeable', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })

      await term.addButton().click()
      const firstActive = term.activePane()
      await expect(firstActive).toBeVisible({ timeout: 10000 })
      const term1Key = (await firstActive.getAttribute('data-terminal-key')) ?? ''
      const term1 = term.pane(term1Key)

      await firstActive.click()
      await win.keyboard.type('echo READY_A')
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await term1.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 600, 1000]
        })
        .toContain('READY_A')

      await win.keyboard.type('for i in 1 2 3 4 5 6; do echo BG_LINE_A_$i; sleep 0.3; done')
      await win.keyboard.press('Enter')

      await term.addButton().click()
      const secondActive = term.activePane()
      await expect(secondActive).toBeVisible()
      await expect(term1).toHaveAttribute('data-active', 'false')

      await win.waitForTimeout(2600)

      await term.item(1).click()
      await expect(term1).toHaveAttribute('data-active', 'true')

      await expect
        .poll(async () => (await term1.textContent()) ?? '', {
          timeout: 10000,
          intervals: [300, 600, 1000]
        })
        .toContain('BG_LINE_A_6')

      expect((await term1.textContent()) ?? '').toContain('READY_A')

      await win.keyboard.type('echo TYPED_AFTER_SWITCH_A')
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await term1.textContent()) ?? '', {
          timeout: 10000,
          intervals: [300, 600, 1000]
        })
        .toContain('TYPED_AFTER_SWITCH_A')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
