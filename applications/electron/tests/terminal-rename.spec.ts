import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

const realShell = (): string =>
  process.env.SHELL && process.env.SHELL.length > 0 ? process.env.SHELL : '/bin/bash'

const sidebarItems = '[data-test="sidebar-terminal-item"]'
const renameInput = '[data-test="sidebar-terminal-rename-input"]'

test.describe('#1609 rename a terminal — inline-edit the label, persist by terminal_id, fall back to auto', () => {
  test('a renamed terminal keeps its custom name across a window reload (persisted by stable terminal_id)', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: realShell() }
    })
    try {
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const item = win.locator(sidebarItems).first()
      await expect(item).toBeVisible({ timeout: 10000 })
      await expect(item).toHaveAttribute('data-state', 'ready', { timeout: 15000 })

      await item.dblclick()
      const input = win.locator(renameInput)
      await expect(input).toBeVisible()
      await input.fill('buildbox')
      await input.press('Enter')

      await expect(win.locator(sidebarItems).first()).toHaveText('1 · buildbox')

      await win.reload()

      const reattached = win.locator(sidebarItems).first()
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
      const addTerminal = win.locator('[data-test="sidebar-add-terminal"]')
      await expect(addTerminal).toBeVisible({ timeout: 20000 })
      await addTerminal.click()

      const item = win.locator(sidebarItems).first()
      await expect(item).toBeVisible({ timeout: 10000 })
      await expect(item).toHaveAttribute('data-state', 'ready', { timeout: 15000 })
      const autoLabel = (await item.textContent()) ?? ''
      expect(autoLabel.startsWith('1 · ')).toBe(true)

      await item.dblclick()
      await win.locator(renameInput).fill('temp')
      await win.locator(renameInput).press('Enter')
      await expect(win.locator(sidebarItems).first()).toHaveText('1 · temp')

      await win.locator(sidebarItems).first().dblclick()
      await win.locator(renameInput).fill('')
      await win.locator(renameInput).press('Enter')
      await expect(win.locator(sidebarItems).first()).toHaveText(autoLabel)

      await win.locator(sidebarItems).first().dblclick()
      await win.locator(renameInput).fill('discard-me')
      await win.locator(renameInput).press('Escape')
      const afterEscape = win.locator(sidebarItems).first()
      await expect(afterEscape).toHaveText(autoLabel)
      await expect(afterEscape).not.toHaveText('1 · discard-me')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
