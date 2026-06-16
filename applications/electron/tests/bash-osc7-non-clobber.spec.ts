import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'

test.describe('#1606 bash OSC-7 non-clobbering — a bash terminal whose own .bashrc sets PROMPT_COMMAND keeps BOTH the live-cwd header AND the user prompt-command on cd', () => {
  test('with /bin/bash and a user PROMPT_COMMAND in .bashrc, cd drives the header to follow (our OSC-7 survived the user rc) while the user prompt-command still runs (our append did not clobber it)', async () => {
    const bash = '/bin/bash'
    const home = mkdtempSync(join(tmpdir(), 'bpmux-bashrc-home-'))
    writeFileSync(join(home, '.bashrc'), "PROMPT_COMMAND='echo BPMUX_USER_PC_MARKER'\n")
    const targetDir = mkdtempSync(join(tmpdir(), 'bpmux-bashcwd-'))
    const targetMarker = targetDir.split('/').filter(Boolean).pop() ?? targetDir

    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: bash, HOME: home }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const activeTerminal = win.locator('[data-test^="terminal-slot-"][data-active="true"]')
      await expect(activeTerminal).toBeVisible({ timeout: 10000 })
      await activeTerminal.click()

      const activeHeader = activeTerminal.locator('[data-test="terminal-header"]')

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

      await expect
        .poll(async () => (await activeTerminal.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 500, 800, 1200],
          message:
            'the user PROMPT_COMMAND (echo BPMUX_USER_PC_MARKER) must still run after cd — our OSC-7 rcfile sources the user rc then APPENDS to PROMPT_COMMAND, so it must never clobber the user prompt-command'
        })
        .toContain('BPMUX_USER_PC_MARKER')
    } finally {
      await app.close()
      await mockGh.close()
      rmSync(home, { recursive: true, force: true })
      rmSync(targetDir, { recursive: true, force: true })
    }
  })
})
