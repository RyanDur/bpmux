import { launchWithMockGh } from './helpers/launch'
import { ChatPage } from './pages/chatPage'
import { expect, test } from '@playwright/test'

test.describe('human-side E2E #476 — direct-action — chat input Enter to send', () => {
  test('typing text + pressing Enter posts the message and clears the input', async () => {
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

      const sendMarker = 'CHAT_INPUT_ENTER_SEND_MARKER_BPF14'
      await textarea.fill(sendMarker)
      await expect(textarea).toHaveValue(sendMarker)

      await textarea.press('Enter')

      await expect(textarea).toHaveValue('')

      const list = chat.messageLog()
      await expect(list).toContainText(sendMarker, { timeout: 10000 })

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Shift+Enter inserts a newline instead of sending', async () => {
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

      await textarea.fill('first line')
      await textarea.press('Shift+Enter')
      await textarea.pressSequentially('second line')

      await expect(textarea).toHaveValue('first line\nsecond line')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — direct-action — chat input addressee parsing', () => {
  test('text starting @<role> renders the addressee marker on the sent message', async () => {
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

      await textarea.fill('@dev addressee parsing test marker BPF15')
      await textarea.press('Enter')
      await expect(textarea).toHaveValue('')

      const list = chat.messageLog()
      await expect(list).toContainText('addressee parsing test marker BPF15', { timeout: 10000 })

      const addresseeMarker = chat.addresseeMarkers().first()
      await expect(addresseeMarker).toBeVisible()
      await expect(addresseeMarker).toHaveText('@dev')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('non-@ text sends without an addressee marker', async () => {
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

      const marker = 'plain broadcast no addressee marker BPF15B'
      await textarea.fill(marker)
      await textarea.press('Enter')

      const list = chat.messageLog()
      await expect(list).toContainText(marker, { timeout: 10000 })

      await expect(chat.addresseeMarkers()).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

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

test.describe('human-side E2E #476 — direct-action — chat input incremental search', () => {
  test('Ctrl+R activates i-search; typing matches a prior message', async () => {
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

      const haystack = 'searchable haystack BPF17'
      await textarea.fill(haystack)
      await textarea.press('Enter')
      await expect(textarea).toHaveValue('')

      await textarea.press('Control+r')
      const isearch = chat.isearchIndicator()
      await expect(isearch).toBeVisible()
      await expect(isearch).toContainText('i-search:')

      await textarea.pressSequentially('haystack')

      await expect(textarea).toHaveValue(haystack)
      await expect(isearch).toContainText('i-search: haystack')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('i-search shows failing-search indicator when query has no match', async () => {
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

      await textarea.fill('hello world')
      await textarea.press('Enter')

      await textarea.press('Control+r')
      const isearch = chat.isearchIndicator()
      await expect(isearch).toBeVisible()

      await textarea.pressSequentially('zzznotpresent')
      await expect(isearch).toContainText('failing i-search')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — persistence-class — chat draft', () => {
  test('typed-but-unsent draft survives window reload, per-channel', async () => {
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

      const draftMarker = 'DRAFT_PERSIST_MARKER_BPF8'
      await textarea.fill(draftMarker)
      await expect(textarea).toHaveValue(draftMarker)

      await win.reload()

      const textareaAfterReload = chat.input()
      await expect(textareaAfterReload).toBeVisible()
      await expect(textareaAfterReload).toHaveValue(draftMarker)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('cleared draft does not resurrect after reload', async () => {
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

      await textarea.fill('TEMPORARY_TEXT')
      await expect(textarea).toHaveValue('TEMPORARY_TEXT')
      await textarea.fill('')
      await expect(textarea).toHaveValue('')

      await win.reload()

      const textareaAfterReload = chat.input()
      await expect(textareaAfterReload).toBeVisible()
      await expect(textareaAfterReload).toHaveValue('')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
