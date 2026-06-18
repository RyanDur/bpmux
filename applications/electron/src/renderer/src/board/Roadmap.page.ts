import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const card = (n: number): HTMLElement => screen.getByTestId(`roadmap-outcome-${n}`)
const drill = (n: number): HTMLElement => screen.getByTestId(`roadmap-drill-${n}`)

export const roadmap = {
  bandHeadings: (): (string | null)[] =>
    screen.getAllByRole('heading', { level: 3 }).map((h) => h.textContent),
  band: (horizon: string): HTMLElement => screen.getByTestId(`roadmap-band-${horizon}`),
  card,
  why: (n: number): HTMLElement | null => within(card(n)).queryByTestId('roadmap-outcome-why'),
  readiness: (n: number): HTMLElement | null =>
    within(card(n)).queryByTestId('roadmap-outcome-readiness'),
  isShown: (id: string): boolean => screen.queryByTestId(id) !== null,
  focus: (n: number): Promise<void> =>
    userEvent.click(screen.getByTestId(`roadmap-outcome-toggle-${n}`)),
  drill,
  drillIteration: (n: number): HTMLElement => screen.getByTestId(`roadmap-drill-iteration-${n}`),
  drillIterationNames: (n: number): (string | null)[] =>
    within(drill(n))
      .getAllByTestId(/roadmap-drill-iteration-\d+/)
      .map((row) => within(row).getByTestId('roadmap-drill-iteration-name').textContent),
  convene: (n: number): string =>
    within(drill(n)).getByTestId(`roadmap-convene-${n}`).textContent ?? ''
}
