import { type Locator, type Page } from '@playwright/test'

export class DrillInPage {
  constructor(private readonly win: Page) {}

  panel(): Locator {
    return this.win.getByRole('article', { name: /^Story \d+:/ })
  }

  acceptButton(): Locator {
    return this.win.getByRole('button', { name: 'Accept', exact: true })
  }

  rejectButton(): Locator {
    return this.win.getByRole('button', { name: 'Reject', exact: true })
  }

  rejectDialog(): Locator {
    return this.win.getByRole('dialog', { name: 'Reject story' })
  }

  rejectNote(): Locator {
    return this.win.getByRole('textbox', { name: /reason/i })
  }

  rejectSend(): Locator {
    return this.win.getByRole('button', { name: 'Send', exact: true })
  }

  cancelReject(): Locator {
    return this.win.getByRole('button', { name: 'Cancel', exact: true })
  }

  backButton(): Locator {
    return this.win.getByRole('button', { name: /back to board/i })
  }

  commentInput(): Locator {
    return this.win.getByRole('textbox', { name: /add a comment/i })
  }

  commentSend(): Locator {
    return this.win.getByRole('button', { name: 'Comment', exact: true })
  }
}
