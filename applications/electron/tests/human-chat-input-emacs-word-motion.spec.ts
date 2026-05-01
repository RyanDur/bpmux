import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

const getCaret = (win: import('@playwright/test').Page): Promise<number> =>
  win.evaluate(() => {
    const ta = document.querySelector(
      '[data-test="chat-input-textarea"]'
    ) as HTMLTextAreaElement | null
    return ta?.selectionStart ?? -1
  })

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

test.describe('human-side E2E #476 — direct-action — chat input emacs word-motion', () => {
  test('Alt+B moves caret to previous word boundary', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
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

      await setCaret(win, 'hello big world'.length)
      await ta.press('Alt+b')
      expect(await getCaret(win)).toBe('hello big '.length)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Alt+F moves caret to next word boundary', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
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

      await setCaret(win, 0)
      await ta.press('Alt+f')
      expect(await getCaret(win)).toBe('hello'.length)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Alt+D kills the next word', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
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

      await setCaret(win, 'hello '.length)
      await ta.press('Alt+d')
      await expect(ta).toHaveValue('hello  world')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
