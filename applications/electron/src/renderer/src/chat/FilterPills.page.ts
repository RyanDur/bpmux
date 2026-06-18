import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { type MessageClass } from '../hooks/useMessageFilter'

const root = (): HTMLElement => screen.getByTestId('filter-pills')
const pill = (cls: MessageClass): HTMLElement => screen.getByTestId(`filter-pill-${cls}`)
const checkbox = (cls: MessageClass): HTMLInputElement =>
  within(pill(cls)).getByRole<HTMLInputElement>('checkbox')

export const filterPills = {
  isRendered: (cls: string): boolean => screen.queryByTestId(`filter-pill-${cls}`) !== null,
  isOn: (cls: MessageClass): boolean => checkbox(cls).checked,
  state: (cls: MessageClass): string | null => pill(cls).getAttribute('data-state'),
  isMarkedEmpty: (cls: MessageClass): boolean => pill(cls).getAttribute('data-empty') === 'true',
  count: (cls: MessageClass): string | null =>
    screen.getByTestId(`filter-pill-${cls}-count`).textContent,
  checkboxLabel: (cls: MessageClass): string | null => checkbox(cls).getAttribute('aria-label'),
  emptyHintShown: (): boolean => screen.queryByTestId('filter-pills-empty-hint') !== null,
  toggle: (cls: MessageClass): Promise<void> => userEvent.click(pill(cls)),
  divCount: (): number => root().querySelectorAll('div').length,
  spanCount: (): number => root().querySelectorAll('span').length,
  hasSwitchRole: (): boolean => within(root()).queryByRole('switch') !== null,
  groupLegend: (): string | null =>
    root().querySelector('fieldset.filter-pills-row legend')?.textContent ?? null
}
