import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, chmodSync, readFileSync, existsSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'

const isAlive = (pid: number): boolean => {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

test.describe('#1596 close/kill (S5) — closing a terminal reaps its process and clears the pane', () => {
  test('clicking the sidebar ✕ closes the terminal: the process is reaped (no orphan), the row + slot are gone, and the center is not left blank', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'bpmux-close-'))
    const pidFile = join(dir, 'agent.pid')
    const agent = join(dir, 'long-agent.sh')
    writeFileSync(agent, '#!/bin/sh\necho $$ > "$BPMUX_AGENT_PIDFILE"\nexec sleep 1000\n')
    chmodSync(agent, 0o755)

    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: agent, BPMUX_AGENT_PIDFILE: pidFile }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const row = win.locator('[data-test="sidebar-terminal-item"]')
      await expect(row).toHaveCount(1, { timeout: 10000 })

      await expect
        .poll(() => (existsSync(pidFile) ? Number(readFileSync(pidFile, 'utf8').trim()) : 0), {
          timeout: 20000,
          intervals: [300, 500, 800, 1200]
        })
        .toBeGreaterThan(0)
      const pid = Number(readFileSync(pidFile, 'utf8').trim())

      expect(isAlive(pid), `the spawned agent (pid ${pid}) must be alive before close`).toBe(true)

      await win.locator('[data-test="sidebar-terminal-close"]').click()

      await expect(row).toHaveCount(0, { timeout: 10000 })
      await expect(win.locator('[data-test^="terminal-slot-"]')).toHaveCount(0)
      await expect(win.locator('[data-test="message-list"]')).toBeVisible()

      await expect
        .poll(() => isAlive(pid), { timeout: 15000, intervals: [300, 500, 800, 1200] })
        .toBe(false)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('closing the ACTIVE terminal when a sibling exists focuses the survivor — not a null/blank center', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'bpmux-focusnext-'))
    const agent = join(dir, 'idle-agent.sh')
    writeFileSync(agent, '#!/bin/sh\nexec sleep 1000\n')
    chmodSync(agent, 0o755)

    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: agent }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()
      await addTerminal.click()

      const items = win.locator('[data-test="sidebar-terminal-item"]')
      await expect(items).toHaveCount(2, { timeout: 10000 })

      await items.nth(0).click()
      const firstActive = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(firstActive).toHaveCount(1)
      const firstSlot = await firstActive.getAttribute('data-test')

      await win.locator('[data-test="sidebar-terminal-close"]').nth(0).click()

      await expect(items).toHaveCount(1, { timeout: 10000 })
      await expect(win.locator('[data-test^="terminal-slot-"]')).toHaveCount(1)
      const survivorActive = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(survivorActive).toHaveCount(1)
      expect(await survivorActive.getAttribute('data-test')).not.toBe(firstSlot)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
