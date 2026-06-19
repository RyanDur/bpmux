import { type Locator, type Page } from '@playwright/test'

export class ChatPage {
  constructor(private readonly win: Page) {}

  appRoot(): Locator {
    return this.win.getByRole('main')
  }

  region(): Locator {
    return this.win.getByRole('region', { name: 'Chat' })
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

  loadingOlderPill(): Locator {
    return this.win.getByText(/loading older messages/i)
  }

  messageRow(text: string): Locator {
    return this.messageLog().getByRole('listitem').filter({ hasText: text })
  }

  messageRows(): Locator {
    return this.messageLog().getByRole('listitem')
  }

  channelTags(): Locator {
    return this.messageLog().locator('.message-channel')
  }

  tooltips(): Locator {
    return this.messageLog().getByRole('tooltip')
  }

  avatars(): Locator {
    return this.messageLog().locator('.message-avatar')
  }

  messageListWrap(): Locator {
    return this.win.locator('.message-list-wrap')
  }

  unfurlContainer(): Locator {
    return this.messageLog().locator('.unfurl-cards')
  }

  unfurlCards(): Locator {
    return this.messageLog().locator('.unfurl-card')
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

  reactionAddButton(): Locator {
    return this.messageLog().getByRole('button', { name: 'Add reaction' })
  }

  reactionPicker(): Locator {
    return this.win.getByRole('menu', { name: 'Pick a reaction' })
  }

  reactionPickerItem(emoji: string): Locator {
    return this.reactionPicker().getByRole('menuitem', { name: `React with ${emoji}` })
  }

  async reactToFirstMessage(emoji: string): Promise<void> {
    await this.messageRows().first().hover()
    await this.reactionAddButton().first().click()
    await this.reactionPickerItem(emoji).first().click()
  }

  reactionPill(accessibleName: string): Locator {
    return this.win.getByRole('button', { name: accessibleName })
  }

  mentionPill(role: string): Locator {
    return this.messageLog().getByRole('button', { name: `@${role}`, exact: true })
  }
}
