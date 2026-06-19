import { type Locator, type Page } from '@playwright/test'

export class TitlebarPage {
  constructor(private readonly win: Page) {}

  connectionStatus(): Locator {
    return this.win.getByLabel(/^bpmux (online|reconnecting|offline)$/)
  }

  appStateText(): Locator {
    return this.win.locator('.titlebar-app-state')
  }

  ciGlyph(): Locator {
    return this.win.getByRole('link', { name: /^CI/ })
  }

  themeToggle(): Locator {
    return this.win.getByRole('button', { name: /^Switch to (light|dark) theme$/ })
  }
}
