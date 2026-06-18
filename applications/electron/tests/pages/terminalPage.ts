import { type Locator, type Page } from '@playwright/test'

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
}
