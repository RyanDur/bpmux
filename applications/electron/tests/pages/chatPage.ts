import { type Locator, type Page } from '@playwright/test'

export class ChatPage {
  constructor(private readonly win: Page) {}

  appRoot(): Locator {
    return this.win.locator('[data-test="app-root"]')
  }

  input(): Locator {
    return this.win.getByRole('textbox', { name: 'chat message' })
  }

  messageLog(): Locator {
    return this.win.getByRole('log', { name: 'Chat messages' })
  }

  messageScroller(): Locator {
    return this.messageLog().getByRole('list')
  }

  messageRow(text: string): Locator {
    return this.messageLog().getByRole('listitem').filter({ hasText: text })
  }

  newMessagesPill(): Locator {
    return this.win.getByRole('button', { name: /new message/i })
  }

  filterRegion(): Locator {
    return this.win.getByRole('complementary', { name: 'Message class filter' })
  }

  connectionState(): Locator {
    return this.win.getByText(/^(online|reconnecting|offline)$/)
  }

  connectionWarning(): Locator {
    return this.win.getByText(/^(connecting|reconnecting)…$/)
  }

  addresseeMarkers(): Locator {
    return this.messageLog().getByTitle(/^attention: @/)
  }

  isearchIndicator(): Locator {
    return this.win.getByText(/i-search:/)
  }

  terminalOutput(): Locator {
    return this.win.getByRole('region', { name: 'terminal output' })
  }

  filterPill(cls: string): Locator {
    return this.win.getByRole('checkbox', { name: new RegExp(`^${cls} messages`) })
  }

  async toggleFilter(cls: string): Promise<void> {
    await this.filterPill(cls).locator('..').click()
  }

  filterEmptyHint(): Locator {
    return this.win.getByText(/All categories muted/i)
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
