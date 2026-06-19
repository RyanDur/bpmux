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

  commentsRegion(): Locator {
    return this.win.getByRole('region', { name: 'Comments' })
  }

  commentList(): Locator {
    return this.win.getByRole('list', { name: 'Comments' })
  }

  commentItems(): Locator {
    return this.commentList().getByRole('listitem')
  }

  commentInput(): Locator {
    return this.win.getByRole('textbox', { name: /add a comment/i })
  }

  commentSend(): Locator {
    return this.win.getByRole('button', { name: 'Comment', exact: true })
  }

  toast(): Locator {
    return this.panel().getByRole('status')
  }

  title(): Locator {
    return this.panel().getByRole('heading', { level: 3 }).first()
  }

  timelineSection(): Locator {
    return this.win.getByRole('group', { name: 'Timeline' })
  }

  timelineSummary(): Locator {
    return this.timelineSection().locator(':scope > summary')
  }

  timelineList(): Locator {
    return this.timelineSection().getByRole('list')
  }

  timelineItems(): Locator {
    return this.timelineList().getByRole('listitem')
  }

  timelineToggle(): Locator {
    return this.timelineSection().getByRole('button', {
      name: /show all events|show major gates/i
    })
  }

  reworkItems(): Locator {
    return this.timelineList().locator('.drill-in-timeline-item--rework')
  }
}
