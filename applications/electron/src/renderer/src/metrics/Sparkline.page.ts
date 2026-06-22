import { screen } from '@testing-library/react'

const svgEl = (): HTMLElement | null => screen.queryByRole('img', { name: /WIP trend/i })

export const sparkline = {
  svg: (): HTMLElement | null => svgEl(),
  trendLabel: (): string | null => svgEl()?.getAttribute('aria-label') ?? null,
  dataTable: (): HTMLElement | null => screen.queryByRole('table', { name: /WIP history/i }),
  emptyPlaceholder: (): HTMLElement | null => screen.queryByText(/no data yet/i),
  polylinePointCount: (): number => {
    const points = svgEl()?.querySelector('polyline')?.getAttribute('points')
    return points ? points.trim().split(/\s+/).filter(Boolean).length : 0
  }
}
