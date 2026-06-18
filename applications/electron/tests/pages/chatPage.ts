import { type Locator, type Page } from '@playwright/test'

export class ChatPage {
  constructor(private readonly win: Page) {}

  appRoot(): Locator {
    return this.win.locator('[data-test="app-root"]')
  }

  async reactToFirstMessage(emoji: string): Promise<void> {
    const row = this.win.locator('[data-test="message-row"]').first()
    await row.hover()
    await this.win.locator('[data-test="reaction-add-first"]').first().click()
    await this.win.locator(`[data-test="reaction-picker-${emoji}"]`).first().click()
  }

  reactionPill(accessibleName: string): Locator {
    return this.win.getByRole('button', { name: accessibleName })
  }
}
