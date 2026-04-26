import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — cross-process — typed message reaches other agent', () => {
  test('human-typed broadcast appears in /api/messages?as=<peer-role> visibility-filter', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const textarea = win.locator('[data-test="chat-input-textarea"]')
    await expect(textarea).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const broadcastMarker = 'CROSS_PROCESS_BROADCAST_MARKER_BPF19'
    await textarea.fill(broadcastMarker)
    await textarea.press('Enter')
    await expect(textarea).toHaveValue('')

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toContainText(broadcastMarker, { timeout: 10000 })

    for (const peer of ['dev', 'pm', 'designer']) {
      const resp = await fetch(
        `http://127.0.0.1:${backendPort}/api/messages?as=${peer}&limit=200`
      )
      expect(resp.status).toBe(200)
      const messages = (await resp.json()) as Array<{ text: string }>
      expect(
        messages.some((m) => m.text === broadcastMarker),
        `peer "${peer}" should observe the broadcast in their visibility-filtered log`
      ).toBe(true)
    }

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('DM scoped to dev is visible to dev but NOT to pm via visibility-filter', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'seed dev as direct-role',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        }
      ]
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const textarea = win.locator('[data-test="chat-input-textarea"]')
    await expect(textarea).toBeVisible()

    const devButton = win.locator('[data-test="sidebar-direct-dev"]')
    await expect(devButton).toBeVisible({ timeout: 10000 })
    await devButton.click()

    consoleErrors.length = 0
    pageErrors.length = 0

    const dmMarker = 'CROSS_PROCESS_DM_MARKER_BPF19'
    await textarea.fill(dmMarker)
    await textarea.press('Enter')
    await expect(textarea).toHaveValue('')

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toContainText(dmMarker, { timeout: 10000 })

    const devResp = await fetch(
      `http://127.0.0.1:${backendPort}/api/messages?as=dev&limit=200`
    )
    expect(devResp.status).toBe(200)
    const devMessages = (await devResp.json()) as Array<{ text: string }>
    expect(devMessages.some((m) => m.text === dmMarker)).toBe(true)

    const pmResp = await fetch(
      `http://127.0.0.1:${backendPort}/api/messages?as=pm&limit=200`
    )
    expect(pmResp.status).toBe(200)
    const pmMessages = (await pmResp.json()) as Array<{ text: string }>
    expect(pmMessages.some((m) => m.text === dmMarker)).toBe(false)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
