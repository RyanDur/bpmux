import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

const FILL_TEXT = 'first line\nsecond line\nthird line'

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

test.describe('human-side E2E #476 — direct-action — chat input emacs cursor-motion', () => {
  test('Ctrl+A moves caret to line start of current line', async () => {
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
      await ta.fill(FILL_TEXT)

      consoleErrors.length = 0
      pageErrors.length = 0

      const middleOfSecondLine = 'first line\nsecond'.length
      await setCaret(win, middleOfSecondLine)

      await ta.press('Control+a')
      expect(await getCaret(win)).toBe('first line\n'.length)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Ctrl+E moves caret to line end of current line', async () => {
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
      await ta.fill(FILL_TEXT)

      consoleErrors.length = 0
      pageErrors.length = 0

      const startOfSecondLine = 'first line\n'.length
      await setCaret(win, startOfSecondLine)

      await ta.press('Control+e')
      expect(await getCaret(win)).toBe('first line\nsecond line'.length)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Ctrl+B moves caret one character back', async () => {
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
      await ta.fill(FILL_TEXT)

      consoleErrors.length = 0
      pageErrors.length = 0

      await setCaret(win, 5)
      await ta.press('Control+b')
      expect(await getCaret(win)).toBe(4)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Ctrl+F moves caret one character forward', async () => {
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
      await ta.fill(FILL_TEXT)

      consoleErrors.length = 0
      pageErrors.length = 0

      await setCaret(win, 5)
      await ta.press('Control+f')
      expect(await getCaret(win)).toBe(6)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
