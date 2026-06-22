import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

test.describe('#1595 / #1608 multiple terminals — distinct id-keyed sessions, switch without disposing, and a faithful restore across reload', () => {
  test('two terminals are distinct id-keyed sessions; switching toggles active and keeps BOTH panes mounted', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.activePane()).toBeVisible({ timeout: 10000 })
      const key1 = await pt.activeTerminalKey()
      const id1 = await pt.activeTerminalId()

      await pt.spawn()
      await expect(pt.items()).toHaveCount(2)
      const key2 = await pt.activeTerminalKey()
      const id2 = await pt.activeTerminalId()
      expect(key2).not.toBe(key1)
      expect(id2).not.toBe(id1)

      await expect(pt.pane(key1)).toBeAttached()
      await expect(pt.pane(key1)).toHaveAttribute('data-active', 'false')
      await expect(pt.pane(key2)).toHaveAttribute('data-active', 'true')

      const label1 = (await pt.item(1).textContent()) ?? ''
      const label2 = (await pt.item(2).textContent()) ?? ''
      expect(label1).toContain('1 · ')
      expect(label2).toContain('2 · ')
      expect(label1).not.toBe(label2)

      await pt.switchTo(1)
      await expect(pt.pane(key1)).toHaveAttribute('data-active', 'true')
      await expect(pt.pane(key2)).toHaveAttribute('data-active', 'false')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('across a reload the previously-active NON-first terminal is restored as the active one, under its same terminal_id', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.activePane()).toBeVisible({ timeout: 10000 })
      const id1 = await pt.activeTerminalId()

      await pt.spawn()
      await expect(pt.items()).toHaveCount(2)
      const id2 = await pt.activeTerminalId()
      expect(id2).not.toBe(id1)
      await expect(pt.item(1)).not.toHaveAttribute('aria-current', 'true')

      await pt.reload()

      await expect(pt.items()).toHaveCount(2, { timeout: 20000 })
      await expect(pt.activePane()).toBeVisible({ timeout: 20000 })
      await expect(pt.item(1)).not.toHaveAttribute('aria-current', 'true')
      await pt.expectActiveTerminalId(id2)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('a terminal that EXITED before the reload does NOT resurrect: only the live survivor is re-presented', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.activePane()).toBeVisible({ timeout: 10000 })
      const survivorId = await pt.activeTerminalId()

      await pt.spawn()
      await expect(pt.items()).toHaveCount(2)
      await pt.focusActive()
      await pt.runUntilItemLabel(2, 'exit', 'exited')

      await pt.reload()

      await expect(pt.activePane()).toBeVisible({ timeout: 20000 })
      await expect(pt.items()).toHaveCount(1)
      await pt.expectActiveTerminalId(survivorId)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
