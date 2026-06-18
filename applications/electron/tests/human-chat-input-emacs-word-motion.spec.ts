import { test, expect, type Locator } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const getCaret = (input: Locator): Promise<number> =>
  input.evaluate((el) => (el as HTMLTextAreaElement).selectionStart ?? -1)

const setCaret = (input: Locator, pos: number): Promise<void> =>
  input.evaluate((el, p) => {
    const ta = el as HTMLTextAreaElement
    ta.focus()
    ta.setSelectionRange(p, p)
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

      const chat = new ChatPage(win)
      const ta = chat.input()
      await expect(ta).toBeVisible()
      await ta.fill('hello big world')

      consoleErrors.length = 0
      pageErrors.length = 0

      await setCaret(ta, 'hello big world'.length)
      await ta.press('Alt+b')
      expect(await getCaret(ta)).toBe('hello big '.length)

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

      const chat = new ChatPage(win)
      const ta = chat.input()
      await expect(ta).toBeVisible()
      await ta.fill('hello big world')

      consoleErrors.length = 0
      pageErrors.length = 0

      await setCaret(ta, 0)
      await ta.press('Alt+f')
      expect(await getCaret(ta)).toBe('hello'.length)

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

      const chat = new ChatPage(win)
      const ta = chat.input()
      await expect(ta).toBeVisible()
      await ta.fill('hello big world')

      consoleErrors.length = 0
      pageErrors.length = 0

      await setCaret(ta, 'hello '.length)
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
