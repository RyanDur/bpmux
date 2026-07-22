import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { builtBpmuxBinary } from './helpers/bpmux-cli'
import { TerminalPage } from './pages/terminalPage'

test.describe('agent direct send by id — the whole system stood up — an id-targeted message reaches exactly one individual among role-sharing agents', () => {
  test('three agents register via CLI typed into terminal sessions, two sharing a role; a send --to-id reaches only the targeted pane', async () => {
    test.setTimeout(120_000)
    const bash = '/bin/bash'
    const home = mkdtempSync(join(tmpdir(), 'bpmux-idsend-home-'))
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
      const alphaId = await term.activeTerminalId()
      await term.runEchoVerified(
        `mkdir -p ~/agent-pm-alpha && cd ~/agent-pm-alpha && export BPMUX_ROLE=pm BPMUX_USER_ID=pm-alpha`
      )
      await term.runEchoVerified(`${bin} register`)
      await term.run('cat')

      await term.addButton().click()
      await term.waitReady()
      const betaId = await term.activeTerminalId()
      await term.runEchoVerified(
        `mkdir -p ~/agent-pm-beta && cd ~/agent-pm-beta && export BPMUX_ROLE=pm BPMUX_USER_ID=pm-beta`
      )
      await term.runEchoVerified(`${bin} register`)
      await term.run('cat')

      const marker = 'ONLY-FOR-ALPHA-9021'
      const squash = (t: string): string => t.replace(/\s+/g, '')
      await term.item(1).click()
      await term.expectActiveTerminalId(senderId)
      await term.runEchoVerified(`${bin} send --to-id pm-alpha "${marker}"`)
      await term.run('cat')

      const barrier = 'BARRIER-ID-SEND-9021'
      const resp = await fetch(`http://127.0.0.1:${backendPort}/api/messages`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ from: 'owner', text: barrier })
      })
      expect(resp.ok).toBe(true)

      await term.item(2).click()
      await term.expectActiveTerminalId(alphaId)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message:
            'the TARGETED individual (pm-alpha) must receive the direct send as an injected ⟦bpmux⟧ line'
        })
        .toContain(`direct▸${marker}`)

      await term.item(3).click()
      await term.expectActiveTerminalId(betaId)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message:
            'the barrier must land in the same-role sibling pane, proving delivery reached it after the direct send'
        })
        .toContain(`broadcast▸${barrier}`)
      const betaText = squash((await term.activePane().textContent()) ?? '')
      expect(
        betaText,
        'the SAME-ROLE sibling (pm-beta, also role pm) never sees the id-targeted message — id is the identity, role is the hat'
      ).not.toContain(marker)

      await term.item(1).click()
      await term.expectActiveTerminalId(senderId)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message: 'the barrier must land in the sender pane too'
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
