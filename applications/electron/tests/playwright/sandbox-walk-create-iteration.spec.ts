import { expect, test } from '@playwright/test'
import { mkdirSync, existsSync } from 'fs'
import { join } from 'path'
import { launchIsolatedWithWindow } from '../helpers/launch'

test.describe('#1564 — live-walk harness slice 1/2: boot + seed + drive create-iteration + capture', () => {
  test('the minimal complete walk-loop on an isolated in-memory sandbox', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      const authToken = await win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )

      const themeName = `SandboxWalk-${Date.now()}`
      const create = await fetch(`http://127.0.0.1:${backendPort}/api/test/create_iteration`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${authToken}` },
        body: JSON.stringify({ theme_name: themeName, agent: 'pm:test' })
      })
      expect(create.ok, 'create_iteration must return 200; got ' + create.status).toBe(true)
      const body = await create.json()
      expect(body.message, body.error ?? 'no message in response').toMatch(/iteration created: #\d+ Iteration /)

      await win.locator('label[data-test="board-tab-plan"]').first().click()
      await win
        .locator('[data-test="board-tab-plan"] input[type="radio"]:checked')
        .first()
        .waitFor({ timeout: 5_000 })
      await expect(win.locator(`text=Iteration ${themeName}`).first()).toBeVisible({
        timeout: 15_000
      })

      const captureDir = join(__dirname, '__captures__')
      if (!existsSync(captureDir)) mkdirSync(captureDir, { recursive: true })
      const capturePath = join(captureDir, 'sandbox-walk-create-iteration.png')
      await win.screenshot({ path: capturePath, fullPage: true })

      expect(backendPort).not.toBe(4117)

      const ownerSidecarCheck = await win.evaluate(
        () =>
          (window as unknown as { bpmux?: { wsUrl?: string; apiBase?: string } }).bpmux?.apiBase ??
          ''
      )
      expect(
        ownerSidecarCheck,
        'sandbox window must point at its ephemeral port, not owner port 4117'
      ).toContain(`:${backendPort}`)
    } finally {
      await app.close()
    }
  })
})
