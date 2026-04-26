import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — titlebar connection status', () => {
  test('connection status renders with one of online/connecting/offline label', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const titlebar = win.locator('[data-test="titlebar"]')
    await expect(titlebar).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const connection = win.locator('[data-test="titlebar-connection"]')
    await expect(connection).toBeVisible()

    const label = win.locator('[data-test="titlebar-connection-label"]')
    await expect(label).toBeVisible()
    const labelText = (await label.textContent())?.trim() ?? ''
    expect(['online', 'connecting…', 'offline']).toContain(labelText)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('connection eventually reaches online state in test-mode', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="titlebar"]')).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const label = win.locator('[data-test="titlebar-connection-label"]')
    await expect(label).toContainText('online', { timeout: 10000 })

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
