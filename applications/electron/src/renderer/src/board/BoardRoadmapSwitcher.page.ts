import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

type View = 'board' | 'roadmap'

const roadmapRadio = (): HTMLInputElement =>
  screen.getByRole<HTMLInputElement>('radio', { name: /Roadmap/i })

const radio = (which: View): HTMLInputElement =>
  which === 'roadmap'
    ? roadmapRadio()
    : screen.getAllByRole<HTMLInputElement>('radio').filter((r) => r !== roadmapRadio())[0]

const tabLabel = (which: View): HTMLElement => {
  const [label] = Array.from(radio(which).labels ?? [])
  if (label === undefined) throw new Error(`no label found for the ${which} tab`)
  return label
}

const nudgeName = /next iteration readiness/i

export const boardRoadmapSwitcher = {
  isActive: (which: View): boolean => radio(which).checked,
  selectTab: (which: View): Promise<void> => userEvent.click(radio(which)),
  tabText: (which: View): string => tabLabel(which).textContent ?? '',
  tabHasText: (which: View, text: string | RegExp): boolean =>
    within(tabLabel(which)).queryByText(text) !== null,
  attnShown: (): boolean => within(tabLabel('roadmap')).queryByText('◐') !== null,
  nudge: (): HTMLElement => screen.getByRole('complementary', { name: nudgeName }),
  isNudgeShown: (): boolean => screen.queryByRole('complementary', { name: nudgeName }) !== null,
  dismissNudge: (): Promise<void> =>
    userEvent.click(screen.getByRole('button', { name: 'Dismiss' })),
  viewRoadmapFromNudge: (): Promise<void> =>
    userEvent.click(screen.getByRole('button', { name: /view roadmap/i }))
}
