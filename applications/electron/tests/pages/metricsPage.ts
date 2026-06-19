import { type Locator, type Page } from '@playwright/test'

export class MetricsPage {
  constructor(private readonly win: Page) {}

  dialog(): Locator {
    return this.win.locator('dialog.metrics')
  }

  overlay(): Locator {
    return this.win.getByRole('dialog', { name: 'Metrics' })
  }

  heading(): Locator {
    return this.overlay().getByRole('heading', { name: 'Velocity + DORA' })
  }

  closeButton(): Locator {
    return this.dialog().getByRole('button', { name: 'Close metrics' })
  }
}
