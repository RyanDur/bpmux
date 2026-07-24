import { screen, within, type waitForOptions } from '@testing-library/react'

const STORY_NAVIGATE = /^Open story #/

const storyIdOf = (button: HTMLElement): number =>
  Number(/#(\d+)/.exec(button.getAttribute('aria-label') ?? '')?.[1])

const iterationCard = (id: number, options?: waitForOptions): Promise<HTMLElement> =>
  screen
    .findByRole(
      'button',
      { name: new RegExp(`view iteration details: #${id}\\b`), hidden: true },
      options
    )
    .then((button) => button.closest('.board-iteration-card') as HTMLElement)

const memberButton = (container: HTMLElement, storyId: number): HTMLButtonElement =>
  within(container).getByRole<HTMLButtonElement>('button', {
    name: new RegExp(`^Open story #${storyId}\\b`),
    hidden: true
  })

export const keyboardReorder = {
  iterationCard,
  memberButton,
  iterationGrip: (id: number, options?: waitForOptions): Promise<HTMLButtonElement> =>
    screen.findByRole<HTMLButtonElement>(
      'button',
      { name: new RegExp(`Reorder iteration #${id}\\b`), hidden: true },
      options
    ),
  announce: (): HTMLElement => screen.getByRole('status', { name: /reorder announcements/i }),
  storyRow: (storyId: number, options?: waitForOptions): Promise<HTMLElement> =>
    screen
      .findByRole(
        'button',
        { name: new RegExp(`^Open story #${storyId}\\b`), hidden: true },
        options
      )
      .then((button) => button.closest('li') as HTMLElement),
  memberOrder: (container: HTMLElement): number[] =>
    within(container).getAllByRole('button', { name: STORY_NAVIGATE, hidden: true }).map(storyIdOf),
  grip: (row: HTMLElement): HTMLElement | null =>
    within(row).queryByRole('button', { name: /^Drag to reorder #/ }),
  navigate: (row: HTMLElement): HTMLElement | null =>
    within(row).queryByRole('button', { name: STORY_NAVIGATE }),
  drillIn: (): HTMLElement | null => screen.queryByRole('article', { name: /^Story \d+:/ }),
  findText: (matcher: RegExp, options?: waitForOptions): Promise<HTMLElement> =>
    screen.findByText(matcher, undefined, options),
  messageList: (): HTMLElement => screen.getByRole('log', { name: /chat messages/i }),
  messageRows: (): HTMLElement[] =>
    within(screen.getByRole('log', { name: /chat messages/i })).getAllByRole('listitem')
}
