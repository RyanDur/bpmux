import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const LONG = 15_000

const MENTION = /^@\w/
const AVATAR = /—/

const messageLog = (): HTMLElement => screen.getByRole('log', { name: /chat messages/i })

const pillCheckbox = (cls: string): HTMLInputElement =>
  screen.getByRole<HTMLInputElement>('checkbox', { name: new RegExp(`^${cls} messages`) })

export const humanMessage = {
  waitForText: (pattern: RegExp): Promise<HTMLElement> =>
    screen.findByText(pattern, undefined, { timeout: LONG }),

  mentions: (): Promise<HTMLElement[]> =>
    screen.findAllByRole('button', { name: MENTION }, { timeout: LONG }),

  mentionText: (mention: HTMLElement): string => mention.textContent ?? '',

  clickFirstMention: (): Promise<void> =>
    userEvent.click(screen.getAllByRole('button', { name: MENTION })[0]),

  sidebarAll: (): Promise<HTMLElement> =>
    screen.findByRole('button', { name: 'all conversations' }, { timeout: LONG }),

  allChannelCurrent: (): string | null =>
    screen.getByRole('button', { name: 'all conversations' }).getAttribute('aria-current'),

  directChannelCurrent: (role: string): string | null =>
    screen.getByRole('button', { name: new RegExp(`^${role}\\b`) }).getAttribute('aria-current'),

  filterPills: (): Promise<HTMLElement> =>
    screen.findByRole('group', { name: /message class/i }, { timeout: LONG }),

  pill: (cls: string): Promise<HTMLElement> =>
    screen.findByRole('checkbox', { name: new RegExp(`^${cls} messages`) }, { timeout: LONG }),

  pillChecked: (cls: string): boolean => pillCheckbox(cls).checked,

  togglePill: (pill: HTMLElement): Promise<void> => userEvent.click(pill),

  reactionAddButton: async (): Promise<HTMLElement> =>
    (await screen.findAllByRole('button', { name: 'Add reaction' }, { timeout: LONG }))[0],

  reactionPickerOpen: (button: HTMLElement): boolean =>
    button.getAttribute('aria-expanded') === 'true',

  openReactionPicker: (button: HTMLElement): Promise<void> => userEvent.click(button),

  reactionPicker: (): Promise<HTMLElement> =>
    screen.findByRole('menu', { name: /pick a reaction/i }, { timeout: 5_000 }),

  reactionPickerCount: (): number =>
    screen.queryAllByRole('menu', { name: /pick a reaction/i }).length,

  pickReaction: async (emoji: string): Promise<void> => {
    const option = await screen.findByRole(
      'menuitem',
      { name: `React with ${emoji}` },
      { timeout: 5_000 }
    )
    await userEvent.click(option)
  },

  reactionPill: (emoji: string): Promise<HTMLElement> =>
    screen.findByRole('button', { name: new RegExp(`^${emoji} `) }, { timeout: LONG }),

  firstRowCount: (): number => within(messageLog()).getAllByRole('button', { name: AVATAR }).length,

  groupedRowCount: (): number =>
    within(messageLog()).getAllByRole('listitem').length -
    within(messageLog()).getAllByRole('button', { name: AVATAR }).length
}
