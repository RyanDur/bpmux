import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { type MessageClass } from './useMessageFilter'

const pill = (cls: MessageClass): HTMLInputElement =>
  screen.getByRole<HTMLInputElement>('checkbox', { name: new RegExp(`^${cls} messages`) })

export const filterPills = {
  isRendered: (cls: string): boolean =>
    screen.queryByRole('checkbox', { name: new RegExp(`^${cls} messages`) }) !== null,
  isOn: (cls: MessageClass): boolean => pill(cls).checked,
  hasLabel: (pattern: RegExp): boolean => screen.queryByLabelText(pattern) !== null,
  toggle: (cls: MessageClass): Promise<void> => userEvent.click(pill(cls)),
  emptyHintShown: (hint: string | RegExp): boolean => screen.queryByText(hint) !== null,
  group: (name: RegExp): HTMLElement | null => screen.queryByRole('group', { name })
}
