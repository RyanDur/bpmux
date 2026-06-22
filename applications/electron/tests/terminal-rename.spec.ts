import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('#1609 rename a terminal — inline-edit the label, persist by terminal_id, fall back to auto', () => {
  test('clearing the custom name falls back to the auto label, and Escape cancels without committing', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.item(1)).toBeVisible({ timeout: 10000 })
      await pt.waitReady()
      await expect(pt.item(1)).toHaveText(/^1 · \S/)
      const autoLabel = (await pt.item(1).textContent()) ?? ''

      await pt.rename(1, 'temp')
      await expect(pt.item(1)).toHaveText('1 · temp')

      await pt.clearName(1)
      await expect(pt.item(1)).toHaveText(autoLabel)

      await pt.renameThenCancel(1, 'discard-me')
      await expect(pt.item(1)).toHaveText(autoLabel)
      await expect(pt.item(1)).not.toHaveText('1 · discard-me')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('keyboard: F2 opens rename with focus in the field (type immediately), Enter commits and returns focus to the terminal item', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await expect(pt.item(1)).toBeVisible({ timeout: 10000 })
      await pt.waitReady()

      await pt.openRenameViaKeyboard(1)
      await expect(pt.renameInput(1)).toBeFocused()

      await win.keyboard.type('keebs')
      await win.keyboard.press('Enter')

      await expect(pt.item(1)).toHaveText('1 · keebs')
      await expect(pt.item(1)).toBeFocused()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
