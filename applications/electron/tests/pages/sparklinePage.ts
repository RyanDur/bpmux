import { type Locator, type Page } from '@playwright/test'

export class SparklinePage {
  constructor(private readonly win: Page) {}

  svg(): Locator {
    return this.win.getByRole('img', { name: /WIP trend/i })
  }

  polyline(): Locator {
    return this.svg().locator('polyline')
  }

  endpointDot(): Locator {
    return this.svg().locator('circle')
  }

  dataTable(): Locator {
    return this.win.getByRole('table', { name: /WIP history/i })
  }

  emptyPlaceholder(): Locator {
    return this.win.getByText(/no data yet/i)
  }

  async polylinePointCount(): Promise<number> {
    const points = await this.polyline().getAttribute('points')
    if (!points) return 0
    return points.trim().split(/\s+/).filter(Boolean).length
  }
}
