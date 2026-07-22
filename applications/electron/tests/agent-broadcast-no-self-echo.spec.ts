import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { builtBpmuxBinary } from './helpers/bpmux-cli'
import { TerminalPage } from './pages/terminalPage'

test.describe('agent broadcast routing — the whole system stood up — a broadcast reaches the peer pane and never echoes into the sender pane', () => {
  test('two agents register via CLI typed into their terminal sessions; one broadcasts; the other pane shows the line, the sender pane does not', async () => {
    test.setTimeout(120_000)
    const bash = '/bin/bash'
    const home = mkdtempSync(join(tmpdir(), 'bpmux-echo-home-'))
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
      await term.runEchoVerified(`mkdir -p ~/agent-pm && cd ~/agent-pm && export BPMUX_ROLE=pm`)
      await term.runEchoVerified(`${bin} register`)
      await term.run('cat')

      const marker = 'NO-SELF-ECHO-MARKER-4417'
      const squash = (t: string): string => t.replace(/\s+/g, '')
      await term.item(1).click()
      await term.expectActiveTerminalId(senderId)
      await term.runEchoVerified(`${bin} send "${marker}"`)
      await term.run('cat')

      await expect
        .poll(
          async () =>
            await (await fetch(`http://127.0.0.1:${backendPort}/api/messages?limit=10`)).text(),
          {
            timeout: 10000,
            message:
              'the broadcast must arrive in the server log — splits CLI->server from server->pane'
          }
        )
        .toContain(marker)

      await term.item(2).click()
      await term.expectActiveTerminalId(peerId)
      await expect
        .poll(async () => squash((await term.activePane().textContent()) ?? ''), {
          timeout: 20000,
          intervals: [400, 700, 1000, 1500],
          message:
            'the OTHER agent pane must receive the broadcast as an injected ⟦bpmux⟧ line — the peer door sees the message'
        })
        .toContain(`broadcast▸${marker}`)

      const barrier = 'BARRIER-AFTER-BROADCAST-4417'
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
            'the owner barrier must land in the sender pane, proving the delivery cycle advanced past the broadcast'
        })
        .toContain(`broadcast▸${barrier}`)

      const senderText = squash((await term.activePane().textContent()) ?? '')
      expect(
        senderText,
        'NO SELF-ECHO: the sender pane shows its typed command and the send confirmation, but NEVER the ⟦bpmux⟧-framed injection of its own broadcast'
      ).not.toContain(`broadcast▸${marker}`)
    } finally {
      await app.close()
      await mockGh.close()
      rmSync(home, { recursive: true, force: true })
    }
  })
})
