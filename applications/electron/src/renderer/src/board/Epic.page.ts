import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const sortButton = (): HTMLElement => screen.getByRole('button', { name: /sort epics by/i })

const epicNumberOf = (viewDetailsButton: HTMLElement): number =>
  Number(/#(\d+)/.exec(viewDetailsButton.getAttribute('aria-label') ?? '')?.[1])

export const epic = {
  viewDetails: (number: number): Promise<void> =>
    userEvent.click(
      screen.getByRole('button', { name: new RegExp(`view epic details: #${number}\\b`) })
    ),
  sortMode: (): string => (sortButton().textContent ?? '').replace(/^sort:\s*/, ''),
  toggleSort: (): Promise<void> => userEvent.click(sortButton()),
  cardOrder: (): number[] =>
    screen.getAllByRole('button', { name: /^view epic details:/i, hidden: true }).map(epicNumberOf)
}
