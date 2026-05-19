import type { UserEvent } from '@testing-library/user-event'

export type ReorderDirection = 'ArrowUp' | 'ArrowDown'

export const pressReorderShortcut = async (
  user: UserEvent,
  target: HTMLElement,
  direction: ReorderDirection
): Promise<void> => {
  target.focus()
  await user.keyboard(`{Control>}{Shift>}{${direction}}{/Shift}{/Control}`)
}
