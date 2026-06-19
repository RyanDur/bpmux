import { type Locator, type Page } from '@playwright/test'

type BoardTab = 'Now' | 'Plan' | 'Done'

export class BoardPage {
  constructor(private readonly win: Page) {}

  region(): Locator {
    return this.win.getByRole('complementary', { name: 'Board' })
  }

  resizeHandle(): Locator {
    return this.win.getByRole('slider', { name: 'Board panel width' })
  }

  tab(name: BoardTab): Locator {
    return this.win.getByRole('radio', { name })
  }

  async openTab(name: BoardTab): Promise<void> {
    await this.win
      .getByRole('group', { name: 'Board sections' })
      .getByText(name, { exact: true })
      .click()
  }

  storyNavigate(storyNumber: number): Locator {
    return this.win.getByRole('button', { name: new RegExp(`Open story #${storyNumber}\\b`) })
  }

  storyRow(storyNumber: number): Locator {
    return this.storyNavigate(storyNumber).locator('xpath=ancestor::li[1]')
  }

  nowContent(): Locator {
    return this.win.getByRole('article', { name: 'Now' })
  }

  async openStory(storyNumber: number): Promise<void> {
    await this.storyNavigate(storyNumber).click({ position: { x: 4, y: 4 } })
  }

  group(label: string): Locator {
    return this.win.getByRole('group', { name: label, exact: true })
  }

  columnToggle(label: string): Locator {
    return this.group(label).locator(':scope > summary')
  }

  columnCount(label: string): Locator {
    return this.group(label).locator('.board-group-count')
  }

  storyInColumn(label: string, storyNumber: number): Locator {
    return this.group(label).locator(`[aria-label^="Open story #${storyNumber} "]`)
  }

  storyAnywhere(storyNumber: number): Locator {
    return this.win.locator(`[aria-label^="Open story #${storyNumber} "]`)
  }

  storyBadge(column: string, label: string): Locator {
    return this.group(column).getByLabel(new RegExp(`^${label}:`))
  }

  iterationGrip(iterationNumber: number): Locator {
    return this.win.getByLabel(new RegExp(`^Reorder iteration #${iterationNumber}\\b`))
  }

  iterationRow(iterationNumber: number): Locator {
    return this.iterationGrip(iterationNumber).locator('xpath=ancestor::article[1]')
  }
}
