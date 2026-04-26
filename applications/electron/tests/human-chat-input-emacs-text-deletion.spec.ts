import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

const setCaret = (win: import('@playwright/test').Page, pos: number): Promise<void> =>
  win.evaluate((p) => {
    const ta = document.querySelector(
      '[data-test="chat-input-textarea"]'
    ) as HTMLTextAreaElement | null
    if (ta) {
      ta.focus()
      ta.setSelectionRange(p, p)
    }
  }, pos)

test.describe('human-side E2E #476 — direct-action — chat input emacs text-deletion', () => {
  test('Ctrl+D deletes one character forward', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const ta = win.locator('[data-test="chat-input-textarea"]')
    await expect(ta).toBeVisible()
    await ta.fill('hello world')

    consoleErrors.length = 0
    pageErrors.length = 0

    await setCaret(win, 5)
    await ta.press('Control+d')
    await expect(ta).toHaveValue('helloworld')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('Ctrl+K kills from caret to end of line', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const ta = win.locator('[data-test="chat-input-textarea"]')
    await expect(ta).toBeVisible()
    await ta.fill('alpha\nbeta\ngamma')

    consoleErrors.length = 0
    pageErrors.length = 0

    const middleOfBeta = 'alpha\nbe'.length
    await setCaret(win, middleOfBeta)

    await ta.press('Control+k')
    await expect(ta).toHaveValue('alpha\nbe\ngamma')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('Ctrl+U kills from line start to caret', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const ta = win.locator('[data-test="chat-input-textarea"]')
    await expect(ta).toBeVisible()
    await ta.fill('alpha\nbeta\ngamma')

    consoleErrors.length = 0
    pageErrors.length = 0

    const middleOfBeta = 'alpha\nbe'.length
    await setCaret(win, middleOfBeta)

    await ta.press('Control+u')
    await expect(ta).toHaveValue('alpha\nta\ngamma')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('Ctrl+W kills the previous word', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const ta = win.locator('[data-test="chat-input-textarea"]')
    await expect(ta).toBeVisible()
    await ta.fill('hello big world')

    consoleErrors.length = 0
    pageErrors.length = 0

    await setCaret(win, 'hello big'.length)

    await ta.press('Control+w')
    await expect(ta).toHaveValue('hello  world')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
