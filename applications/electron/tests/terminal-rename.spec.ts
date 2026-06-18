import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

test.describe('#1609 rename a terminal — inline-edit the label, persist by terminal_id, fall back to auto', () => {
  test('a renamed terminal keeps its custom name across a window reload (persisted by stable terminal_id)', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const item = term.item(1)
      await expect(item).toBeVisible({ timeout: 10000 })
      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await item.dblclick()
      const input = term.renameInput(1)
      await expect(input).toBeVisible()
      await input.fill('buildbox')
      await input.press('Enter')

      await expect(term.item(1)).toHaveText('1 · buildbox')

      await win.reload()

      const reattached = term.item(1)
      await expect(reattached).toBeVisible({ timeout: 20000 })
      await expect(reattached).toHaveText('1 · buildbox')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('clearing the custom name falls back to the auto label, and Escape cancels without committing', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const item = term.item(1)
      await expect(item).toBeVisible({ timeout: 10000 })
      await expect(term.output()).toBeVisible({ timeout: 15000 })
      const autoLabel = (await item.textContent()) ?? ''
      expect(autoLabel.startsWith('1 · ')).toBe(true)

      await item.dblclick()
      await term.renameInput(1).fill('temp')
      await term.renameInput(1).press('Enter')
      await expect(term.item(1)).toHaveText('1 · temp')

      await term.item(1).dblclick()
      await term.renameInput(1).fill('')
      await term.renameInput(1).press('Enter')
      await expect(term.item(1)).toHaveText(autoLabel)

      await term.item(1).dblclick()
      await term.renameInput(1).fill('discard-me')
      await term.renameInput(1).press('Escape')
      const afterEscape = term.item(1)
      await expect(afterEscape).toHaveText(autoLabel)
      await expect(afterEscape).not.toHaveText('1 · discard-me')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('keyboard: F2 opens rename with focus in the field (type immediately), Enter commits and returns focus to the terminal item', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const term = new TerminalPage(win)
      await expect(term.addButton()).toBeVisible({ timeout: 20000 })
      await term.addButton().click()

      const item = term.item(1)
      await expect(item).toBeVisible({ timeout: 10000 })
      await expect(term.output()).toBeVisible({ timeout: 15000 })

      await item.press('F2')
      const input = term.renameInput(1)
      await expect(input).toBeFocused()

      await win.keyboard.type('keebs')
      await win.keyboard.press('Enter')

      const renamed = term.item(1)
      await expect(renamed).toHaveText('1 · keebs')
      await expect(renamed).toBeFocused()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
