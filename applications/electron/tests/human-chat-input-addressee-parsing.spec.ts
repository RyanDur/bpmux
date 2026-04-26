import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — chat input addressee parsing', () => {
  test('text starting @<role> renders the addressee marker on the sent message', async () => {
    const { app, win } = await launchIsolatedWithWindow({
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

    await textarea.fill('@dev addressee parsing test marker BPF15')
    await textarea.press('Enter')
    await expect(textarea).toHaveValue('')

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toContainText('addressee parsing test marker BPF15', { timeout: 10000 })

    const addresseeMarker = list.locator('.message-addressee').first()
    await expect(addresseeMarker).toBeVisible()
    await expect(addresseeMarker).toHaveText('@dev')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('non-@ text sends without an addressee marker', async () => {
    const { app, win } = await launchIsolatedWithWindow({
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

    const marker = 'plain broadcast no addressee marker BPF15B'
    await textarea.fill(marker)
    await textarea.press('Enter')

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toContainText(marker, { timeout: 10000 })

    const addresseeMarkers = list.locator('.message-addressee')
    await expect(addresseeMarkers).toHaveCount(0)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
