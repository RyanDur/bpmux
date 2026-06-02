import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

const slots = '[data-test^="terminal-slot-"]'
const activeSlot = '[data-test^="terminal-slot-"][data-active="true"]'
const sidebarItems = '[data-test="sidebar-terminal-item"]'

test.describe('#1608 terminals survive refresh — re-attach by terminal_id (no vanish, no resurrect)', () => {
  test('after a window reload a terminal re-attaches as the SAME live session: its cwd and its background job persist (re-attach, not reap+respawn)', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const active = win.locator(activeSlot)
      await expect(active).toBeVisible({ timeout: 10000 })
      await active.click()

      await win.keyboard.type(
        'cd /tmp && mkdir -p bpmux-1608-marker && cd bpmux-1608-marker && echo CWD_READY_1608'
      )
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await active.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 600, 1000]
        })
        .toContain('CWD_READY_1608')

      await win.keyboard.type('sleep 999 &')
      await win.keyboard.press('Enter')
      await win.keyboard.type('echo JOB_STARTED_1608')
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await active.textContent()) ?? '', {
          timeout: 10000,
          intervals: [300, 600, 1000]
        })
        .toContain('JOB_STARTED_1608')

      await win.reload()

      const reattachedAsSameLiveSession = win.locator(slots).first()
      await expect(reattachedAsSameLiveSession).toBeVisible({ timeout: 20000 })
      await expect(win.locator(sidebarItems)).toHaveCount(1)
      await reattachedAsSameLiveSession.click()

      await win.keyboard.type('pwd')
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await reattachedAsSameLiveSession.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 600, 1000]
        })
        .toContain('bpmux-1608-marker')

      await win.keyboard.type('jobs')
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await reattachedAsSameLiveSession.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 600, 1000]
        })
        .toContain('sleep 999')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the full SET restores and a terminal that EXITED during the disconnect does NOT resurrect', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })

      await addTerminal.click()
      const terminalA = win.locator(activeSlot)
      await expect(terminalA).toBeVisible({ timeout: 10000 })
      await terminalA.click()
      await win.keyboard.type(
        'cd /tmp && mkdir -p bpmux-1608-A && cd bpmux-1608-A && echo A_READY_1608'
      )
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await terminalA.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 600, 1000]
        })
        .toContain('A_READY_1608')

      await addTerminal.click()
      await expect(win.locator(sidebarItems)).toHaveCount(2)
      const terminalBExitingDuringSession = win.locator(activeSlot)
      await terminalBExitingDuringSession.click()
      await win.keyboard.type('exit')
      await win.keyboard.press('Enter')
      await expect(win.locator('[data-test^="terminal-state-"][data-state="exited"]')).toBeVisible({
        timeout: 15000
      })

      await win.reload()

      const survivor = win.locator(slots).first()
      await expect(survivor).toBeVisible({ timeout: 20000 })
      await expect(win.locator(sidebarItems)).toHaveCount(1)

      await survivor.click()
      await win.keyboard.type('pwd')
      await win.keyboard.press('Enter')
      await expect
        .poll(async () => (await survivor.textContent()) ?? '', {
          timeout: 15000,
          intervals: [300, 600, 1000]
        })
        .toContain('bpmux-1608-A')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
