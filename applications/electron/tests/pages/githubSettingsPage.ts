import { type Locator, type Page } from '@playwright/test'

export class GithubSettingsPage {
  constructor(private readonly win: Page) {}

  openButton(): Locator {
    return this.win.locator('[data-test="titlebar-settings"]')
  }

  dialog(): Locator {
    return this.win.getByRole('dialog', { name: 'GitHub settings' })
  }

  connectionState(): Locator {
    return this.win.locator('[data-test="gh-conn-state"]')
  }

  owner(): Locator {
    return this.win.locator('[data-test="gh-owner"]')
  }

  repo(): Locator {
    return this.win.locator('[data-test="gh-repo"]')
  }

  token(): Locator {
    return this.win.locator('[data-test="gh-token"]')
  }

  enterprise(): Locator {
    return this.win.locator('[data-test="gh-enterprise"]')
  }

  baseUrl(): Locator {
    return this.win.locator('[data-test="gh-base-url"]')
  }

  connect(): Locator {
    return this.win.locator('[data-test="gh-connect"]')
  }

  disconnect(): Locator {
    return this.win.locator('[data-test="gh-disconnect"]')
  }
}
