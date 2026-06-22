import { type Locator, type Page, expect } from '@playwright/test'

export class TerminalPage {
  constructor(private readonly win: Page) {}

  addButton(): Locator {
    return this.win.getByRole('button', { name: 'spawn a new terminal' })
  }

  items(): Locator {
    return this.win.getByRole('button', { name: /^\d+ · / })
  }

  item(n: number): Locator {
    return this.win.getByRole('button', { name: new RegExp(`^${n} · `) })
  }

  closeButton(n: number): Locator {
    return this.win.getByRole('button', { name: `close terminal ${n}` })
  }

  renameInput(n: number): Locator {
    return this.win.getByRole('textbox', { name: `rename terminal ${n}` })
  }

  headerClose(): Locator {
    return this.win.getByRole('button', { name: 'close terminal', exact: true })
  }

  output(): Locator {
    return this.win.getByRole('region', { name: 'terminal output' })
  }

  headerStatus(): Locator {
    return this.win.getByLabel('terminal location and size')
  }

  pane(key: string): Locator {
    return this.win.locator(`.terminal-layer[data-terminal-key="${key}"]`)
  }

  activePane(): Locator {
    return this.win.locator('.terminal-layer[data-active="true"]')
  }

  restartButton(): Locator {
    return this.win.getByRole('button', { name: /^(restart|retry)$/ })
  }

  paneCloseButton(): Locator {
    return this.win.getByRole('button', { name: 'close', exact: true })
  }

  confirmDialog(): Locator {
    return this.win.getByRole('dialog', { name: 'Close terminal?' })
  }

  confirmCancel(): Locator {
    return this.confirmDialog().getByRole('button', { name: 'cancel' })
  }

  confirmAccept(): Locator {
    return this.confirmDialog().getByRole('button', { name: 'close anyway' })
  }

  async activeTerminalId(): Promise<string> {
    await expect(this.activePane()).toHaveAttribute('data-terminal-id', /.+/, { timeout: 15000 })
    const id = await this.activePane().getAttribute('data-terminal-id')
    if (!id) throw new Error('the active terminal pane has no data-terminal-id (not ready?)')
    return id
  }

  async activeTerminalKey(): Promise<string> {
    await expect(this.activePane()).toHaveAttribute('data-terminal-key', /.+/, { timeout: 15000 })
    const key = await this.activePane().getAttribute('data-terminal-key')
    if (!key) throw new Error('the active terminal pane has no data-terminal-key')
    return key
  }

  async expectActiveTerminalId(expected: string): Promise<void> {
    await expect
      .poll(() => this.activePane().getAttribute('data-terminal-id'), {
        timeout: 20000,
        intervals: [200, 400, 800, 1200]
      })
      .toBe(expected)
  }

  paneTerminalId(key: string): Promise<string | null> {
    return this.pane(key).getAttribute('data-terminal-id')
  }

  async spawn(): Promise<void> {
    await expect(this.addButton()).toBeVisible({ timeout: 20000 })
    await this.addButton().click()
  }

  async waitReady(): Promise<void> {
    await expect(this.output()).toBeVisible({ timeout: 15000 })
  }

  async focusActive(): Promise<void> {
    await this.output().click()
  }

  async reload(): Promise<void> {
    await this.win.reload()
  }

  async runUntilItemLabel(n: number, command: string, expectedLabel: string): Promise<void> {
    await expect
      .poll(
        async () => {
          await this.win.keyboard.type(command)
          await this.win.keyboard.press('Enter')
          return (await this.item(n).textContent()) ?? ''
        },
        { timeout: 20000, intervals: [400, 700, 1000, 1500] }
      )
      .toContain(expectedLabel)
  }

  async rename(n: number, name: string): Promise<void> {
    await this.item(n).dblclick()
    const input = this.renameInput(n)
    await expect(input).toBeVisible()
    await input.fill(name)
    await input.press('Enter')
  }

  async clearName(n: number): Promise<void> {
    await this.item(n).dblclick()
    await this.renameInput(n).fill('')
    await this.renameInput(n).press('Enter')
  }

  async renameThenCancel(n: number, text: string): Promise<void> {
    await this.item(n).dblclick()
    await this.renameInput(n).fill(text)
    await this.renameInput(n).press('Escape')
  }

  async openRenameViaKeyboard(n: number): Promise<void> {
    await this.item(n).press('F2')
  }

  async switchTo(n: number): Promise<void> {
    await this.item(n).click()
  }

  async selectOutputRegion(): Promise<void> {
    const box = await this.output().boundingBox()
    if (!box) throw new Error('terminal output has no bounding box')
    await this.win.mouse.move(box.x + 14, box.y + 14)
    await this.win.mouse.down()
    await this.win.mouse.move(box.x + box.width - 14, box.y + box.height - 14, { steps: 16 })
    await this.win.mouse.up()
  }

  async copySelection(): Promise<void> {
    await this.win.keyboard.press('Meta+c')
  }
}
