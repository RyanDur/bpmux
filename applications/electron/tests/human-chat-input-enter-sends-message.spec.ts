import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — chat input Enter to send', () => {
  test('typing text + pressing Enter posts the message and clears the input', async () => {
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

    const sendMarker = 'CHAT_INPUT_ENTER_SEND_MARKER_BPF14'
    await textarea.fill(sendMarker)
    await expect(textarea).toHaveValue(sendMarker)

    await textarea.press('Enter')

    await expect(textarea).toHaveValue('')

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toContainText(sendMarker, { timeout: 10000 })

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('Shift+Enter inserts a newline instead of sending', async () => {
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

    await textarea.fill('first line')
    await textarea.press('Shift+Enter')
    await textarea.pressSequentially('second line')

    await expect(textarea).toHaveValue('first line\nsecond line')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
