import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

type TabName = 'now' | 'plan' | 'done'

const TAB_LABEL: Record<TabName, string> = { now: 'Now', plan: 'Plan', done: 'Done' }

const radio = (name: TabName): HTMLInputElement =>
  screen.getByRole<HTMLInputElement>('radio', { name: TAB_LABEL[name] })

const visiblePanel = (name: TabName): HTMLElement =>
  screen.getByRole('article', { name: TAB_LABEL[name] })

export const boardTabs = {
  hasTab: (label: string): boolean => screen.queryByRole('radio', { name: label }) !== null,
  isActive: (name: TabName): boolean => radio(name).checked,
  isPanelVisible: (name: TabName): boolean =>
    screen.queryByRole('article', { name: TAB_LABEL[name] }) !== null,
  selectTab: (name: TabName): Promise<void> => userEvent.click(radio(name)),
  panelHasStory: (name: TabName, storyId: number): boolean =>
    within(visiblePanel(name)).queryAllByRole('button', { name: new RegExp(`#${storyId}\\b`) })
      .length > 0,
  panelHasGroup: (name: TabName, groupLabel: string): boolean =>
    within(visiblePanel(name)).queryAllByText(groupLabel).length > 0,
  panelHasIteration: (name: TabName, iterationTitle: string): boolean =>
    within(visiblePanel(name)).queryByText(iterationTitle) !== null,
  nowHasRoadmap: (): boolean =>
    within(visiblePanel('now')).queryByText(/live · always current/) !== null,
  epicTitleCount: (title: string): number => screen.queryAllByText(title).length
}
