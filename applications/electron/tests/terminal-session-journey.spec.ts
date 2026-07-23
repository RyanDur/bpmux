import { ChatPage } from './pages/chatPage'
import { TerminalPage } from './pages/terminalPage'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { mkdtempSync, writeFileSync, chmodSync, readFileSync, existsSync } from 'fs'
import { test, expect } from '@playwright/test'
import { tmpdir } from 'os'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

const isAlive = (pid: number): boolean => {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

test.describe('the terminal session journey — spawn, type, rename, switch, reload-restore, exit, restart, and every close affordance', () => {
  test('after a reload the one terminal re-appears under the SAME terminal_id and keeps its renamed label', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await pt.waitReady()
      await expect(pt.item(1)).toHaveText(/^1 · \S/)
      const idBeforeReload = await pt.activeTerminalId()

      await pt.rename(1, 'buildbox')
      await expect(pt.item(1)).toHaveText('1 · buildbox')

      await pt.reload()

      await expect(pt.item(1)).toBeVisible({ timeout: 20000 })
      await expect(pt.items()).toHaveCount(1)
      await expect(pt.item(1)).toHaveText('1 · buildbox')

      await pt.waitReady()
      await pt.expectActiveTerminalId(idBeforeReload)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

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

  test('clearing the custom name falls back to the auto label, and Escape cancels without committing', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.item(1)).toBeVisible({ timeout: 10000 })
      await pt.waitReady()
      await expect(pt.item(1)).toHaveText(/^1 · \S/)
      const autoLabel = (await pt.item(1).textContent()) ?? ''

      await pt.rename(1, 'temp')
      await expect(pt.item(1)).toHaveText('1 · temp')

      await pt.clearName(1)
      await expect(pt.item(1)).toHaveText(autoLabel)

      await pt.renameThenCancel(1, 'discard-me')
      await expect(pt.item(1)).toHaveText(autoLabel)
      await expect(pt.item(1)).not.toHaveText('1 · discard-me')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('keyboard: F2 opens rename with focus in the field (type immediately), Enter commits and returns focus to the terminal item', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.item(1)).toBeVisible({ timeout: 10000 })
      await pt.waitReady()

      await pt.openRenameViaKeyboard(1)
      await expect(pt.renameInput(1)).toBeFocused()

      await win.keyboard.type('keebs')
      await win.keyboard.press('Enter')

      await expect(pt.item(1)).toHaveText('1 · keebs')
      await expect(pt.item(1)).toBeFocused()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('a real shell that EXITS shows "exited" (server PtyExit), not a blank working pane', async () => {
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

      await term.run('exit')

      await expect(term.item(1)).toContainText('exited', { timeout: 15000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('a failed spawn shows "spawn-failed" loudly, never a blank pane', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/nonexistent/bpf1581-bogus-shell' }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      await expect(term.item(1)).toContainText('failed', { timeout: 15000 })

      await expect(win.getByText('spawn-failed')).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('clicking ＋ spawns a real $SHELL terminal: a typed command executes and its output renders (not a printf double, not a blank pane)', async () => {
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

      await expect
        .poll(
          async () => {
            await term.run('echo TERM=$TERM')
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

  test('typing into a ＋-spawned terminal round-trips onData → WS → backend → the PTY stdin', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'bpmux-interact-'))
    const teeFile = join(dir, 'received')
    const agent = join(dir, 'tee-agent.sh')
    writeFileSync(agent, '#!/bin/sh\nexec dd of="$BPMUX_ROOM_TEE_FILE" bs=1 2>/dev/null\n')
    chmodSync(agent, 0o755)

    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: agent, BPMUX_ROOM_TEE_FILE: teeFile }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const activeTerminal = term.output()
      await expect(activeTerminal).toBeVisible({ timeout: 10000 })
      await activeTerminal.click()

      const marker = 'ASSEMBLY_MARKER_BPF1592'
      await expect
        .poll(
          async () => {
            await win.keyboard.type(marker)
            await win.keyboard.press('Enter')
            return existsSync(teeFile) ? readFileSync(teeFile, 'utf8') : ''
          },
          { timeout: 20000, intervals: [400, 700, 1000, 1500] }
        )
        .toContain(marker)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

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
      const term = new TerminalPage(win)
      const chat = new ChatPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const row = term.items()
      await expect(row).toHaveCount(1, { timeout: 10000 })

      await expect
        .poll(() => (existsSync(pidFile) ? Number(readFileSync(pidFile, 'utf8').trim()) : 0), {
          timeout: 20000,
          intervals: [300, 500, 800, 1200]
        })
        .toBeGreaterThan(0)
      const pid = Number(readFileSync(pidFile, 'utf8').trim())

      expect(isAlive(pid), `the spawned agent (pid ${pid}) must be alive before close`).toBe(true)

      await term.closeButton(1).click()

      await expect(row).toHaveCount(0, { timeout: 10000 })
      await expect(term.output()).toHaveCount(0)
      await expect(chat.messageLog()).toBeVisible()

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
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()
      await term.addButton().click()

      await expect(term.items()).toHaveCount(2, { timeout: 10000 })

      await term.item(1).click()
      await expect(term.item(1)).toHaveAttribute('aria-current', 'true')
      await expect(term.output()).toBeVisible()

      await term.closeButton(1).click()

      await expect(term.items()).toHaveCount(1, { timeout: 10000 })
      await expect(term.output()).toBeVisible()
      await expect(term.item(1)).toHaveAttribute('aria-current', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('closing a terminal with a running foreground process prompts to confirm; cancel keeps it, confirm-anyway closes it', async () => {
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
      await term.run('echo OUT:$((6*7))')
      await expect(activeScreen).toContainText('OUT:42', { timeout: 15000 })

      await term.run('sleep 30')
      await win.waitForTimeout(1000)

      const dialog = term.confirmDialog()

      await term.closeButton(1).click()
      await expect(dialog).toBeVisible({ timeout: 5000 })

      await term.confirmCancel().click()
      await expect(dialog).toHaveCount(0)
      await expect(term.items()).toHaveCount(1)

      await term.closeButton(1).click()
      await expect(dialog).toBeVisible({ timeout: 5000 })
      await term.confirmAccept().click()
      await expect(term.items()).toHaveCount(0, { timeout: 10000 })
      await expect(term.output()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('an exited terminal offers restart (re-spawns a live terminal) and close (removes the dead pane)', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await pt.waitReady()

      await pt.focusActive()
      await pt.runUntilItemLabel(1, 'exit', 'exited')

      await expect(pt.restartButton()).toBeVisible()
      await expect(pt.paneCloseButton()).toBeVisible()

      await pt.restartButton().click()
      await pt.waitReady()

      await pt.focusActive()
      await pt.runUntilItemLabel(1, 'exit', 'exited')

      await pt.paneCloseButton().click()
      await expect(pt.items()).toHaveCount(0, { timeout: 10000 })
      await expect(pt.output()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('closing a SPAWNING terminal (before its id arrives) reaps the just-created server PTY on the spawn response — no orphan', async () => {
    test.setTimeout(60_000)
    const dir = mkdtempSync(join(tmpdir(), 'bpmux-spawnclose-'))
    const pidFile = join(dir, 'agent.pid')
    const agent = join(dir, 'pid-agent.sh')
    writeFileSync(agent, '#!/bin/sh\necho $$ > "$BPMUX_AGENT_PIDFILE"\nexec sleep 1000\n')
    chmodSync(agent, 0o755)

    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: agent, BPMUX_AGENT_PIDFILE: pidFile }
    })
    try {
      await win.route('**/api/terminals/spawn', async (route) => {
        const response = await route.fetch()
        await new Promise((resolve) => setTimeout(resolve, 2500))
        await route.fulfill({ response })
      })

      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      await expect(term.item(1)).toContainText('spawning', { timeout: 5000 })

      await expect
        .poll(() => (existsSync(pidFile) ? Number(readFileSync(pidFile, 'utf8').trim()) : 0), {
          timeout: 20000,
          intervals: [200, 400, 700, 1000]
        })
        .toBeGreaterThan(0)
      const pid = Number(readFileSync(pidFile, 'utf8').trim())
      expect(isAlive(pid), `the spawned agent (pid ${pid}) is alive mid-spawn`).toBe(true)

      await term.closeButton(1).click()
      await expect(term.items()).toHaveCount(0, { timeout: 10000 })

      await expect
        .poll(() => isAlive(pid), { timeout: 20000, intervals: [400, 700, 1000, 1500] })
        .toBe(false)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the ✕ in the terminal header tears down a ready terminal (the second close affordance)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
    })
    try {
      const term = new TerminalPage(win)
      const chat = new ChatPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await expect(term.headerClose()).toBeVisible()
      await win.waitForTimeout(2000)
      await term.headerClose().click()

      await expect(term.items()).toHaveCount(0, { timeout: 10000 })
      await expect(term.output()).toHaveCount(0)
      await expect(chat.messageLog()).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
