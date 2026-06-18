import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, chmodSync, readFileSync, existsSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

const isAlive = (pid: number): boolean => {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

test.describe('#1596 close/kill (S5) — closing a terminal mid-spawn does not orphan the server PTY', () => {
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
})
