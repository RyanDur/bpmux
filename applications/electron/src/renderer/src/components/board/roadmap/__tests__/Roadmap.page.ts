import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const toggle = (name: string): HTMLElement => screen.getByRole('button', { name })

const card = (name: string): HTMLElement => toggle(name).closest('.roadmap-outcome') as HTMLElement

const iterationNameEls = (): HTMLElement[] =>
  Array.from(document.querySelectorAll<HTMLElement>('.roadmap-drill-iteration-name'))

const drillRow = (iterationName: string): HTMLElement => {
  const nameEl = iterationNameEls().find((el) => el.textContent === iterationName)
  return nameEl?.closest('.roadmap-drill-iteration') as HTMLElement
}

export const roadmap = {
  bandHeadings: (): (string | null)[] =>
    screen.getAllByRole('heading', { level: 3 }).map((h) => h.textContent),
  band: (label: string): HTMLElement =>
    screen.getByRole('heading', { level: 3, name: label }).closest('.roadmap-band') as HTMLElement,
  card,
  hasBenefit: (name: string): boolean => card(name).querySelector('.roadmap-outcome-why') !== null,
  readiness: (name: string): HTMLElement | null =>
    within(card(name)).queryByText(/^(ready|\d+ gaps?)$/),
  emptyShown: (): boolean => screen.queryByText('No outcomes on the roadmap yet') !== null,
  focus: (name: string): Promise<void> => userEvent.click(toggle(name)),
  drill: (name: string): HTMLElement => card(name).querySelector('.roadmap-drill') as HTMLElement,
  drillShown: (name: string): boolean => card(name).querySelector('.roadmap-drill') !== null,
  iterationShown: (iterationName: string): boolean =>
    iterationNameEls().some((el) => el.textContent === iterationName),
  drillIteration: drillRow,
  checksShown: (iterationName: string): boolean =>
    within(drillRow(iterationName)).queryByLabelText(/: (met|gap|pending)$/) !== null,
  drillIterationNames: (outcomeName: string): string[] =>
    Array.from(card(outcomeName).querySelectorAll('.roadmap-drill-iteration-name')).map(
      (el) => el.textContent ?? ''
    ),
  convene: (outcomeName: string): string =>
    card(outcomeName).querySelector('.roadmap-convene')?.textContent ?? ''
}
