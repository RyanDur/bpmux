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

  async animationName(): Promise<string> {
    return this.polyline().evaluate((el: Element) => getComputedStyle(el).animationName)
  }

  async drawInKeyframes(): Promise<{ exists: boolean; from: number; to: number }> {
    return this.win.evaluate(() => {
      const offset = (frames: CSSKeyframeRule[], keys: string[]): number => {
        const frame = frames.find((f) => keys.includes(f.keyText))
        return Number.parseFloat(frame?.style.getPropertyValue('stroke-dashoffset') ?? '')
      }
      const sheets = [...Array.from(document.styleSheets), ...document.adoptedStyleSheets]
      for (const sheet of sheets) {
        try {
          for (const rule of Array.from(sheet.cssRules)) {
            if (rule instanceof CSSKeyframesRule && rule.name === 'sparkline-draw-in') {
              const frames = Array.from(rule.cssRules) as CSSKeyframeRule[]
              return {
                exists: true,
                from: offset(frames, ['0%', 'from']),
                to: offset(frames, ['100%', 'to'])
              }
            }
          }
        } catch {
          continue
        }
      }
      return { exists: false, from: Number.NaN, to: Number.NaN }
    })
  }
}
