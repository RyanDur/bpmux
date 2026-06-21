import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { runCommandUntilOutput } from './helpers/terminal'
import { TerminalPage } from './pages/terminalPage'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

test.describe('#1608 terminals survive refresh — re-attach by terminal_id (no vanish, no resurrect)', () => {
  test('after a window reload a terminal re-attaches as the SAME live session: its cwd and its background job persist (re-attach, not reap+respawn)', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const active = term.output()
      await expect(active).toBeVisible({ timeout: 10000 })
      await active.click()

      await runCommandUntilOutput(
        win,
        'cd /tmp && mkdir -p bpmux-1608-marker && cd bpmux-1608-marker && echo CWD_READY_1608',
        active,
        'CWD_READY_1608'
      )

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

      const reattachedAsSameLiveSession = term.output()
      await expect(reattachedAsSameLiveSession).toBeVisible({ timeout: 20000 })
      await expect(term.items()).toHaveCount(1)
      await reattachedAsSameLiveSession.click()

      await runCommandUntilOutput(win, 'pwd', reattachedAsSameLiveSession, 'bpmux-1608-marker')

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
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })

      await term.addButton().click()
      const terminalA = term.output()
      await expect(terminalA).toBeVisible({ timeout: 10000 })
      await terminalA.click()
      await runCommandUntilOutput(
        win,
        'cd /tmp && mkdir -p bpmux-1608-A && cd bpmux-1608-A && echo A_READY_1608',
        terminalA,
        'A_READY_1608'
      )

      await term.addButton().click()
      await expect(term.items()).toHaveCount(2)
      const terminalBExitingDuringSession = term.output()
      await terminalBExitingDuringSession.click()
      await runCommandUntilOutput(win, 'exit', term.item(2), 'exited')

      await win.reload()

      const survivor = term.output()
      await expect(survivor).toBeVisible({ timeout: 20000 })
      await expect(term.items()).toHaveCount(1)

      await survivor.click()
      await runCommandUntilOutput(win, 'pwd', survivor, 'bpmux-1608-A')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('with multiple terminals surviving a reload, the previously-active NON-first terminal is restored as the active one', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })

      await term.addButton().click()
      const firstSpawned = term.output()
      await expect(firstSpawned).toBeVisible({ timeout: 10000 })
      await firstSpawned.click()
      await runCommandUntilOutput(
        win,
        'cd /tmp && mkdir -p bpmux-1608-first && cd bpmux-1608-first && echo FIRST_READY_1608',
        firstSpawned,
        'FIRST_READY_1608'
      )

      await term.addButton().click()
      await expect(term.items()).toHaveCount(2)
      const secondSpawned = term.output()
      await expect(secondSpawned).toBeVisible()
      await secondSpawned.click()
      await runCommandUntilOutput(
        win,
        'cd /tmp && mkdir -p bpmux-1608-second && cd bpmux-1608-second && echo SECOND_READY_1608',
        secondSpawned,
        'SECOND_READY_1608'
      )

      await expect(term.item(1)).not.toHaveAttribute('aria-current', 'true')

      await win.reload()

      await expect(term.items()).toHaveCount(2, { timeout: 20000 })
      const restoredActive = term.output()
      await expect(restoredActive).toBeVisible({ timeout: 20000 })
      await expect(term.item(1)).not.toHaveAttribute('aria-current', 'true')

      await restoredActive.click()
      await runCommandUntilOutput(win, 'pwd', restoredActive, 'bpmux-1608-second')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
