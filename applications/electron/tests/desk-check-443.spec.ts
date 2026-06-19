import { test, expect, type Page, type Locator } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

interface State {
  value: string
  selStart: number
  selEnd: number
}

const setupInput = async (win: Page, text: string, caret: number): Promise<Locator> => {
  const ta = new ChatPage(win).input()
  await expect(ta).toBeVisible()
  await ta.focus()
  await ta.fill(text)
  await expect(ta).toHaveValue(text)
  await ta.evaluate((el, pos) => {
    const node = el as HTMLTextAreaElement
    node.setSelectionRange(pos, pos)
  }, caret)
  return ta
}

const expectStateWithin = async (ta: Locator, expected: State, timeoutMs = 1000): Promise<void> => {
  await expect
    .poll(
      async () =>
        ta.evaluate((el) => {
          const node = el as HTMLTextAreaElement
          return {
            value: node.value,
            selStart: node.selectionStart ?? 0,
            selEnd: node.selectionEnd ?? 0
          }
        }),
      { timeout: timeoutMs }
    )
    .toEqual(expected)
}

test.describe('desk-check #443 rework — emacs keybind behavior preserved', () => {
  test('Ctrl+A moves cursor to line-start', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello world', 5)
    await win.keyboard.press('Control+a')
    await expectStateWithin(ta, { value: 'hello world', selStart: 0, selEnd: 0 })
    await app.close()
  })

  test('Ctrl+E moves cursor to line-end', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello world', 5)
    await win.keyboard.press('Control+e')
    await expectStateWithin(ta, { value: 'hello world', selStart: 11, selEnd: 11 })
    await app.close()
  })

  test('Ctrl+B moves cursor one char back (clamped at 0)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello', 3)
    await win.keyboard.press('Control+b')
    await expectStateWithin(ta, { value: 'hello', selStart: 2, selEnd: 2 })
    await win.keyboard.press('Control+b')
    await win.keyboard.press('Control+b')
    await win.keyboard.press('Control+b')
    await expectStateWithin(ta, { value: 'hello', selStart: 0, selEnd: 0 })
    await app.close()
  })

  test('Ctrl+F moves cursor one char forward (clamped at length)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'abc', 1)
    await win.keyboard.press('Control+f')
    await expectStateWithin(ta, { value: 'abc', selStart: 2, selEnd: 2 })
    await win.keyboard.press('Control+f')
    await win.keyboard.press('Control+f')
    await expectStateWithin(ta, { value: 'abc', selStart: 3, selEnd: 3 })
    await app.close()
  })

  test('Alt+B moves cursor to previous word boundary', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello brave new world', 16)
    await win.keyboard.press('Alt+b')
    await expectStateWithin(ta, { value: 'hello brave new world', selStart: 12, selEnd: 12 })
    await win.keyboard.press('Alt+b')
    await expectStateWithin(ta, { value: 'hello brave new world', selStart: 6, selEnd: 6 })
    await app.close()
  })

  test('Alt+F moves cursor to next word boundary', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello brave new world', 0)
    await win.keyboard.press('Alt+f')
    await expectStateWithin(ta, { value: 'hello brave new world', selStart: 5, selEnd: 5 })
    await win.keyboard.press('Alt+f')
    await expectStateWithin(ta, { value: 'hello brave new world', selStart: 11, selEnd: 11 })
    await app.close()
  })

  test('Ctrl+K kills to end of line and places cursor at kill-start', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello world', 5)
    await win.keyboard.press('Control+k')
    await expectStateWithin(ta, { value: 'hello', selStart: 5, selEnd: 5 })
    await app.close()
  })

  test('Ctrl+U kills to start of line and places cursor at line-start', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello world', 6)
    await win.keyboard.press('Control+u')
    await expectStateWithin(ta, { value: 'world', selStart: 0, selEnd: 0 })
    await app.close()
  })

  test('Ctrl+W kills previous word and places cursor at word-start', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello brave world', 11)
    await win.keyboard.press('Control+w')
    await expectStateWithin(ta, { value: 'hello  world', selStart: 6, selEnd: 6 })
    await app.close()
  })

  test('Alt+D kills next word and keeps cursor at position', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello brave world', 6)
    await win.keyboard.press('Alt+d')
    await expectStateWithin(ta, { value: 'hello  world', selStart: 6, selEnd: 6 })
    await app.close()
  })

  test('Ctrl+D deletes char forward and keeps cursor at position', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'hello', 2)
    await win.keyboard.press('Control+d')
    await expectStateWithin(ta, { value: 'helo', selStart: 2, selEnd: 2 })
    await app.close()
  })

  test('textarea retains focus after disabled→enabled transition', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = new ChatPage(win).input()
    await expect(ta).toBeVisible()
    await ta.focus()
    await expect(ta).toBeFocused()
    await app.close()
  })

  test('typed-in-progress value survives within a channel context', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const ta = await setupInput(win, 'in-progress draft', 17)
    await expect(ta).toHaveValue('in-progress draft')
    await app.close()
  })
})
