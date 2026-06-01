import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

test.describe('#1466 real-$SHELL assembly — spawning a terminal runs a REAL shell whose executed output renders', () => {
  test('opening the dev terminal spawns a real $SHELL: a typed command executes and its output renders (not a printf double, not a blank pane)', async () => {
    const realShell =
      process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'seed dev as direct-role',
          ts: '2026-06-01T16:00:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['dev:claude-code'],
      extraEnv: { BPMUX_ROOM_AGENT: realShell }
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
      await terminal.click()

      await expect
        .poll(
          async () => {
            await win.keyboard.type('echo TERM=$TERM')
            await win.keyboard.press('Enter')
            return (await terminal.textContent()) ?? ''
          },
          { timeout: 20000, intervals: [400, 700, 1000, 1500] }
        )
        .toContain('TERM=xterm-256color')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
