import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, chmodSync, readFileSync, existsSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

test.describe('#1592 interact-assembly — typed keystrokes reach the agent through the wired terminal', () => {
  test('typing into the dev terminal round-trips onData → WS → backend → the dev PTY stdin', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'bpmux-interact-'))
    const teeFile = join(dir, 'received')
    const agent = join(dir, 'tee-agent.sh')
    writeFileSync(agent, '#!/bin/sh\nexec dd of="$BPMUX_ROOM_TEE_FILE" bs=1 2>/dev/null\n')
    chmodSync(agent, 0o755)

    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'seed dev as direct-role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['dev:claude-code'],
      extraEnv: { BPMUX_ROOM_AGENT: agent, BPMUX_ROOM_TEE_FILE: teeFile }
    })
    try {
      const sidebar = win.locator('[data-test="sidebar"]')
      await expect(sidebar).toBeVisible()
      await waitForMessagesLoaded(win, 'seed dev as direct-role')

      const devButton = win.locator('[data-test="sidebar-direct-dev"]')
      await expect(devButton).toBeVisible({ timeout: 20000 })
      await devButton.click()

      const terminal = win.locator('[data-test="terminal-dev"]')
      await expect(terminal).toBeVisible({ timeout: 10000 })

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
