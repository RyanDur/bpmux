import { type Locator, type Page } from '@playwright/test'

export class TitlebarPage {
  constructor(private readonly win: Page) {}

  ciGlyph(): Locator {
    return this.win.getByRole('link', { name: /^CI/ })
  }
}
