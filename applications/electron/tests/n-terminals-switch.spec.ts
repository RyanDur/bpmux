import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1595 N terminals — spawn two, switch, and neither is disposed (scrollback/running-state persist)', () => {
  test('＋ spawns distinct id-keyed terminals; switching toggles active and keeps BOTH mounted (not disposed)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })

      await addTerminal.click()
      const firstActive = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(firstActive).toBeVisible({ timeout: 10000 })
      const term1Slot = await firstActive.getAttribute('data-test')

      await addTerminal.click()
      await expect(win.locator('[data-test="sidebar-terminal-item"]')).toHaveCount(2)

      const secondActive = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(secondActive).toBeVisible()
      const term2Slot = await secondActive.getAttribute('data-test')
      expect(term2Slot).not.toBe(term1Slot)

      const term1 = win.locator(`[data-test="${term1Slot}"]`)
      await expect(term1).toBeAttached()
      await expect(term1).toHaveAttribute('data-active', 'false')

      await win.locator('[data-test="sidebar-terminal-item"]').nth(0).click()
      await expect(term1).toHaveAttribute('data-active', 'true')

      const term2 = win.locator(`[data-test="${term2Slot}"]`)
      await expect(term2).toBeAttached()
      await expect(term2).toHaveAttribute('data-active', 'false')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
