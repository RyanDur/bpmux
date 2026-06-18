import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

test.describe('human-side E2E #476 — direct-action — chat input history recall', () => {
  test('ArrowUp recalls the most recent sent message; ArrowDown returns to draft', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chat = new ChatPage(win)
      const textarea = chat.input()
      await expect(textarea).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const sentText = 'history recall marker BPF16'
      await textarea.fill(sentText)
      await textarea.press('Enter')
      await expect(textarea).toHaveValue('')

      await textarea.press('ArrowUp')
      await expect(textarea).toHaveValue(sentText)

      await textarea.press('ArrowDown')
      await expect(textarea).toHaveValue('')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Ctrl+P / Ctrl+N navigate history same as ArrowUp / ArrowDown', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const chat = new ChatPage(win)
      const textarea = chat.input()
      await expect(textarea).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const a = 'first sent BPF16A'
      const b = 'second sent BPF16B'
      await textarea.fill(a)
      await textarea.press('Enter')
      await textarea.fill(b)
      await textarea.press('Enter')

      await textarea.press('Control+p')
      await expect(textarea).toHaveValue(b)

      await textarea.press('Control+p')
      await expect(textarea).toHaveValue(a)

      await textarea.press('Control+n')
      await expect(textarea).toHaveValue(b)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
