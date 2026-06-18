import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1597 live-cwd header (S6) — the terminal HEADER tracks cwd as you cd, the SIDEBAR label stays static', () => {
  test('cd in a real $SHELL drives OSC-7 → the active header follows to the new dir while the sidebar label keeps its stable spawn-dir identity', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const targetDir = mkdtempSync(join(tmpdir(), 'bpmux-livecwd-'))
    const targetMarker = targetDir.split('/').filter(Boolean).pop() ?? targetDir

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

      const activeHeader = term.headerStatus()
      const sidebarLabel = term.item(1)
      const spawnDirLabel = (await sidebarLabel.textContent()) ?? ''

      await expect
        .poll(
          async () => {
            await win.keyboard.type(`cd ${targetDir}`)
            await win.keyboard.press('Enter')
            return (await activeHeader.textContent()) ?? ''
          },
          { timeout: 20000, intervals: [400, 700, 1000, 1500] }
        )
        .toContain(targetMarker)

      expect(
        (await sidebarLabel.textContent()) ?? '',
        'the sidebar label must keep its stable spawn-dir identity — it must NOT follow the cd (identity is a stable handle, not a moving target)'
      ).toBe(spawnDirLabel)
      expect((await sidebarLabel.textContent()) ?? '').not.toContain(targetMarker)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
