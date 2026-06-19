import { type Locator, type Page } from '@playwright/test'

export class SyncStatusPage {
  constructor(private readonly win: Page) {}

  glyph(): Locator {
    return this.win.getByRole('status', { name: /^Sync state:/ })
  }
}
