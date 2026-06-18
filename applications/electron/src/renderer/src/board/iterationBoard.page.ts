import { fireEvent, screen, within, type waitForOptions } from '@testing-library/react'

const STORY_NAVIGATE = /^Open story #/

const storyNumberOf = (button: HTMLElement): number =>
  Number(/#(\d+)/.exec(button.getAttribute('aria-label') ?? '')?.[1])

const cardOf = (button: HTMLElement): HTMLElement =>
  button.closest('.board-iteration-card') as HTMLElement

const rowOf = (button: HTMLElement): HTMLElement => button.closest('li') as HTMLElement

type SyntheticDataTransfer = Pick<
  DataTransfer,
  'setData' | 'getData' | 'types' | 'effectAllowed' | 'dropEffect'
>

export interface DragGesture {
  dataTransfer: SyntheticDataTransfer
  start: (grip: HTMLElement) => void
  over: (target: HTMLElement) => void
  drop: (target: HTMLElement) => void
  end: (grip: HTMLElement) => void
  dropEffect: () => string
}

export const newDragGesture = (): DragGesture => {
  const dataStore: Record<string, string> = {}
  const dataTransfer: SyntheticDataTransfer = {
    setData: (type: string, value: string) => {
      dataStore[type] = value
    },
    getData: (type: string) => dataStore[type] ?? '',
    types: [] as unknown as readonly string[],
    effectAllowed: 'move',
    dropEffect: 'move'
  }
  return {
    dataTransfer,
    start: (grip) => {
      fireEvent.dragStart(grip, { dataTransfer })
      Object.defineProperty(dataTransfer, 'types', { value: Object.keys(dataStore) })
    },
    over: (target) => fireEvent.dragOver(target, { dataTransfer }),
    drop: (target) => fireEvent.drop(target, { dataTransfer }),
    end: (grip) => fireEvent.dragEnd(grip, { dataTransfer }),
    dropEffect: () => dataTransfer.dropEffect
  }
}

const memberButton = (container: HTMLElement, n: number): HTMLElement =>
  within(container).getByRole('button', {
    name: new RegExp(`^Open story #${n}\\b`),
    hidden: true
  })

export const iterationBoard = {
  container: (n: number, _legacy?: unknown, waitOptions?: waitForOptions): Promise<HTMLElement> =>
    screen
      .findByRole(
        'button',
        { name: new RegExp(`view iteration details: #${n}\\b`), hidden: true },
        waitOptions
      )
      .then(cardOf),
  unscheduled: (_legacy?: unknown, waitOptions?: waitForOptions): Promise<HTMLElement> =>
    screen
      .findByText('Unscheduled', undefined, waitOptions)
      .then((title) => title.closest('.board-iteration-card') as HTMLElement),
  storyRow: (container: HTMLElement, n: number): HTMLElement => rowOf(memberButton(container, n)),
  memberRow: (container: HTMLElement, n: number): HTMLLIElement =>
    rowOf(memberButton(container, n)) as HTMLLIElement,
  grip: (row: HTMLElement): HTMLElement =>
    within(row).getByRole('button', { name: /^Drag to reorder #/ }),
  summary: (container: HTMLElement): HTMLElement =>
    container.querySelector(':scope > summary') as HTMLElement,
  memberOrder: (container: HTMLElement): number[] =>
    within(container)
      .getAllByRole('button', { name: STORY_NAVIGATE, hidden: true })
      .map(storyNumberOf),
  closedBadge: (row: HTMLElement): HTMLElement | null => within(row).queryByText('(closed)'),
  iterationOrder: (): number[] =>
    screen
      .getAllByRole('button', { name: /^view iteration details: #/i, hidden: true })
      .map(storyNumberOf)
}
