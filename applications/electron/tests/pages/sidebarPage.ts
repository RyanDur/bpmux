import { type Locator, type Page } from '@playwright/test'

export class SidebarPage {
  constructor(private readonly win: Page) {}

  nav(): Locator {
    return this.win.getByRole('navigation', { name: 'Channels' })
  }

  allChannel(): Locator {
    return this.nav().getByRole('button', { name: 'all conversations' })
  }

  directChannel(role: string): Locator {
    return this.nav().getByRole('button', { name: new RegExp(`^${role}\\b`) })
  }

  groupChannels(): Locator {
    return this.nav().getByRole('button', { name: /^scope chat to / })
  }

  groupChannel(label: string): Locator {
    return this.nav().getByRole('button', { name: `scope chat to ${label}` })
  }

  presenceDot(role: string): Locator {
    return this.directChannel(role).locator('.presence-dot')
  }

  groupsEmpty(): Locator {
    return this.nav().getByRole('heading', { name: 'groups' }).locator('..').getByText('none yet')
  }
}
