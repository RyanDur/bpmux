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

  boardScrollPanel(): Locator {
    return this.win.getByRole('article', { name: 'Board' })
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

  storyGrip(storyNumber: number): Locator {
    return this.win.getByRole('button', {
      name: new RegExp(`Drag to reorder #${storyNumber}\\b`)
    })
  }

  nowContent(): Locator {
    return this.win.getByRole('article', { name: 'Now' })
  }

  planContent(): Locator {
    return this.win.getByRole('article', { name: 'Plan' })
  }

  tabBar(): Locator {
    return this.win.getByRole('group', { name: 'Board sections' })
  }

  epicsSection(): Locator {
    return this.win.getByRole('region', { name: 'Epics' })
  }

  epicCard(epicNumber: number): Locator {
    return this.win.getByRole('group', { name: new RegExp(`Epic #${epicNumber}\\b`) })
  }

  epicState(epicNumber: number): Locator {
    return this.epicCard(epicNumber).locator('.board-epic-state')
  }

  epicProgress(epicNumber: number): Locator {
    return this.epicCard(epicNumber).getByRole('progressbar')
  }

  epicMiniboard(epicNumber: number): Locator {
    return this.epicCard(epicNumber).locator('.board-epic-miniboard')
  }

  epicMiniColumn(epicNumber: number, columnLabel: string): Locator {
    return this.epicMiniboard(epicNumber).getByLabel(new RegExp(`^${columnLabel}:`))
  }

  epicViewDetails(epicNumber: number): Locator {
    return this.epicCard(epicNumber).getByRole('button', {
      name: new RegExp(`view epic details: #${epicNumber}\\b`)
    })
  }

  epicLink(storyNumber: number): Locator {
    return this.storyRow(storyNumber).getByRole('button', { name: /^open epic #/ })
  }

  closeApproachNudge(): Locator {
    return this.win.getByRole('complementary', { name: 'Next iteration readiness' })
  }

  closeApproachDismiss(): Locator {
    return this.win.getByRole('button', { name: 'Dismiss' })
  }

  roadmapAttn(): Locator {
    return this.win.locator('.board-roadmap-attn')
  }

  headerIteration(): Locator {
    return this.win.locator('.board-iteration-name')
  }

  async unscheduledOrder(): Promise<number[]> {
    return this.win
      .locator('.board-iteration-card--unscheduled .board-story-navigate')
      .evaluateAll((els) =>
        els.map((el) => Number((el.getAttribute('aria-label') ?? '').match(/#(\d+)/)?.[1]))
      )
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

  async openColumn(label: string): Promise<void> {
    const summary = this.columnToggle(label)
    if ((await summary.getAttribute('aria-expanded')) !== 'true') {
      await summary.click()
    }
  }

  storyStateBadge(storyNumber: number): Locator {
    return this.storyRow(storyNumber).locator('.story-badge')
  }

  reorderAnnounce(): Locator {
    return this.win.getByRole('status', { name: 'Reorder announcements' })
  }

  async storyOrder(columnLabel: string): Promise<number[]> {
    return this.group(columnLabel)
      .getByRole('button', { name: /^Open story #/ })
      .evaluateAll((els) =>
        els.map((el) => Number((el.getAttribute('aria-label') ?? '').match(/#(\d+)/)?.[1]))
      )
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

  async memberOrder(iterationNumber: number): Promise<number[]> {
    return this.iterationRow(iterationNumber).evaluate((article) =>
      Array.from(article.querySelectorAll('[data-iteration-member]')).map((el) =>
        Number(el.getAttribute('data-iteration-member'))
      )
    )
  }

  async iterationOrder(): Promise<number[]> {
    return this.win.evaluate(() =>
      Array.from(document.querySelectorAll('[aria-label^="Reorder iteration #"]'))
        .map((el) => (el.getAttribute('aria-label') ?? '').match(/Reorder iteration #(\d+)/))
        .filter((m): m is RegExpMatchArray => m !== null)
        .map((m) => Number(m[1]))
    )
  }

  iterationCard(iterationNumber: number): Locator {
    return this.win.getByRole('group', { name: new RegExp(`Iteration #${iterationNumber}\\b`) })
  }

  iterationSummary(iterationNumber: number): Locator {
    return this.iterationCard(iterationNumber).locator(':scope > summary')
  }

  activePlannedSection(): Locator {
    return this.win.getByRole('group', { name: 'Active + Planned iterations' })
  }

  activePlannedSummary(): Locator {
    return this.activePlannedSection().locator(':scope > summary')
  }

  closedIterationsSection(): Locator {
    return this.win.getByRole('group', { name: 'Closed iterations' })
  }

  async openClosedIterationsSection(): Promise<void> {
    const section = this.closedIterationsSection()
    if ((await section.getAttribute('open')) === null) {
      await section.locator(':scope > summary').click()
    }
  }
}
