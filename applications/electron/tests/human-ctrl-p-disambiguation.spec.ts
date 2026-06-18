import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'

test.describe('human-side E2E #476 — Ctrl+P context-dependent disambiguation', () => {
  test('Ctrl+P with chat-input focused recalls history (NOT toggle board)', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      const chat = new ChatPage(win)
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const textarea = chat.input()
      const marker = 'CTRL_P_HISTORY_MARKER_BPF24'
      await textarea.fill(marker)
      await textarea.press('Enter')
      await expect(textarea).toHaveValue('')

      await textarea.press('Control+p')
      await expect(textarea).toHaveValue(marker)
      await expect(board.region()).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Ctrl+P with focus outside chat-input toggles board panel', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      const chat = new ChatPage(win)
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await chat.connectionState().click()

      await win.keyboard.press('Control+p')
      await expect(board.region()).toBeHidden()

      await win.keyboard.press('Control+p')
      await expect(board.region()).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
