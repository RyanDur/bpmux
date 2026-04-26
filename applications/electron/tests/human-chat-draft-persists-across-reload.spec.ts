import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — persistence-class — chat draft', () => {
  test('typed-but-unsent draft survives window reload, per-channel', async () => {
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

    const draftMarker = 'DRAFT_PERSIST_MARKER_BPF8'
    await textarea.fill(draftMarker)
    await expect(textarea).toHaveValue(draftMarker)

    await win.reload()

    const textareaAfterReload = win.locator('[data-test="chat-input-textarea"]')
    await expect(textareaAfterReload).toBeVisible()
    await expect(textareaAfterReload).toHaveValue(draftMarker)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('cleared draft does not resurrect after reload', async () => {
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

    await textarea.fill('TEMPORARY_TEXT')
    await expect(textarea).toHaveValue('TEMPORARY_TEXT')
    await textarea.fill('')
    await expect(textarea).toHaveValue('')

    await win.reload()

    const textareaAfterReload = win.locator('[data-test="chat-input-textarea"]')
    await expect(textareaAfterReload).toBeVisible()
    await expect(textareaAfterReload).toHaveValue('')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
