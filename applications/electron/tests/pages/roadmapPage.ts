import { type Locator, type Page } from '@playwright/test'

export class RoadmapPage {
  constructor(private readonly win: Page) {}

  viewTab(view: 'Board' | 'Roadmap'): Locator {
    return this.win.getByRole('radio', { name: new RegExp(view, 'i') })
  }

  async selectView(view: 'Board' | 'Roadmap'): Promise<void> {
    await this.viewTab(view).locator('..').click()
  }

  boardTabLabel(): Locator {
    return this.viewTab('Board').locator('..')
  }

  panel(): Locator {
    return this.win.getByRole('article', { name: 'Roadmap' })
  }

  outcome(title: string): Locator {
    return this.win.getByRole('button', { name: title })
  }

  drill(): Locator {
    return this.win.locator('.roadmap-drill')
  }

  iterationOrder(): Locator {
    return this.win.locator('.roadmap-drill-iteration-name')
  }

  convene(): Locator {
    return this.win.locator('.roadmap-convene')
  }

  iterationRow(name: string): Locator {
    return this.drill().getByRole('listitem').filter({ hasText: name })
  }

  checks(iterationName: string): Locator {
    return this.iterationRow(iterationName).getByLabel(/: (met|gap|pending)$/)
  }

  nowRoadmap(): Locator {
    return this.win.getByRole('article', { name: 'Now' }).locator('.roadmap')
  }

  band(label: 'Now' | 'Next' | 'Later'): Locator {
    return this.win.locator('.roadmap-band', {
      has: this.win.getByRole('heading', { level: 3, name: label })
    })
  }

  outcomeInBand(label: 'Now' | 'Next' | 'Later', title: string): Locator {
    return this.band(label).getByRole('button', { name: title })
  }

  async moveToBand(title: string, label: 'Now' | 'Next' | 'Later'): Promise<void> {
    await this.win
      .getByRole('group', { name: `Move ${title} to a horizon` })
      .getByRole('button', { name: label })
      .click()
  }

  headerIteration(): Locator {
    return this.win.locator('.board-iteration-name')
  }
}
