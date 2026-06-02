import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, chmodSync, readFileSync, existsSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1592/#1595 interact-assembly — typed keystrokes reach the ＋-spawned terminal PTY stdin', () => {
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
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const activeTerminal = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
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
})
