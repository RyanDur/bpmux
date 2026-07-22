import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { builtBpmuxBinary } from './helpers/bpmux-cli'
import { TerminalPage } from './pages/terminalPage'

test.describe('agent id discovery — the whole system stood up — who lists linked agents and its id column reaches the listed individual', () => {
  test('two agents register via CLI typed into terminal sessions; who shows the peer id; a send --to-id of that id lands in the peer pane only', async () => {
    test.setTimeout(120_000)
    const bash = '/bin/bash'
    const home = mkdtempSync(join(tmpdir(), 'bpmux-who-home-'))
    writeFileSync(join(home, '.bashrc'), '')
    const bin = builtBpmuxBinary()

    const { app, win, mockGh, backendPort } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: bash, HOME: home }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })

      await term.addButton().click()
      await term.waitReady()
      const senderId = await term.activeTerminalId()
      await term.runEchoVerified(`mkdir -p ~/agent-dev && cd ~/agent-dev && export BPMUX_ROLE=dev`)
      await term.runEchoVerified(`${bin} register`)

      await term.addButton().click()
      await term.waitReady()
      const peerId = await term.activeTerminalId()
      await term.runEchoVerified(
        `mkdir -p ~/agent-pm && cd ~/agent-pm && export BPMUX_ROLE=pm BPMUX_USER_ID=pm-target`
      )
      await term.runEchoVerified(`${bin} register`)
      await term.run('cat')

      const squash = (t: string): string => t.replace(/\s+/g, '')
      await term.item(1).click()
      await term.expectActiveTerminalId(senderId)
      await term.runEchoVerified(`${bin} who`)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message:
            'who typed into a pane must list the peer agent id — discovery without catching register output'
        })
        .toContain('pm-target')

      const marker = 'WHO-THEN-DIRECT-7733'
      await term.runEchoVerified(`${bin} send --to-id pm-target "${marker}"`)
      await term.run('cat')

      await term.item(2).click()
      await term.expectActiveTerminalId(peerId)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message:
            'the id read off who must work verbatim as a --to-id target — the discovered id IS the routing id'
        })
        .toContain(`direct▸${marker}`)

      const barrier = 'BARRIER-WHO-7733'
      const resp = await fetch(`http://127.0.0.1:${backendPort}/api/messages`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ from: 'owner', text: barrier })
      })
      expect(resp.ok).toBe(true)
      await term.item(1).click()
      await term.expectActiveTerminalId(senderId)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message:
            'the owner barrier must land in the sender pane, proving delivery advanced past the direct send'
        })
        .toContain(`broadcast▸${barrier}`)
      const senderText = squash((await term.activePane().textContent()) ?? '')
      expect(
        senderText,
        'the sender pane never receives the ⟦bpmux⟧-framed injection of its own direct send'
      ).not.toContain(`direct▸${marker}`)
    } finally {
      await app.close()
      await mockGh.close()
      rmSync(home, { recursive: true, force: true })
    }
  })
})
