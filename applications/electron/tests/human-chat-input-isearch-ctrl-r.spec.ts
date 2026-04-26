import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — direct-action — chat input incremental search', () => {
  test('Ctrl+R activates i-search; typing matches a prior message', async () => {
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

    const haystack = 'searchable haystack BPF17'
    await textarea.fill(haystack)
    await textarea.press('Enter')
    await expect(textarea).toHaveValue('')

    await textarea.press('Control+r')
    const isearch = win.locator('[data-test="chat-input-isearch"]')
    await expect(isearch).toBeVisible()
    await expect(isearch).toContainText('i-search:')

    await textarea.pressSequentially('haystack')

    await expect(textarea).toHaveValue(haystack)
    await expect(isearch).toContainText('i-search: haystack')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('i-search shows failing-search indicator when query has no match', async () => {
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

    await textarea.fill('hello world')
    await textarea.press('Enter')

    await textarea.press('Control+r')
    const isearch = win.locator('[data-test="chat-input-isearch"]')
    await expect(isearch).toBeVisible()

    await textarea.pressSequentially('zzznotpresent')
    await expect(isearch).toContainText('failing i-search')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
