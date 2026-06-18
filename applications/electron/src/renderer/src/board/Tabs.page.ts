import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const radios = (): HTMLInputElement[] => screen.getAllByRole<HTMLInputElement>('radio')
const panels = (): HTMLElement[] => screen.getAllByRole('article', { hidden: true })

export const tabs = {
  group: (name: string): HTMLElement => screen.getByRole('group', { name }),
  radios,
  radio: (index: number): HTMLInputElement => radios()[index],
  radioCount: (): number => radios().length,
  everyRadioIsNativeInput: (): boolean =>
    radios().every(
      (r) => r.getAttribute('type') === 'radio' && r.tagName.toLowerCase() === 'input'
    ),
  groupName: (): string => radios()[0].name,
  allShareName: (name: string): boolean => radios().every((r) => r.name === name),
  panelCount: (): number => panels().length,
  panelHidden: (index: number): boolean => panels()[index].hidden,
  clickTab: (label: string): Promise<void> =>
    userEvent.click(screen.getByRole('radio', { name: label })),
  clickRadio: (index: number): Promise<void> => userEvent.click(radios()[index]),
  hasTablistRole: (): boolean => screen.queryByRole('tablist') !== null,
  hasTabRole: (): boolean => screen.queryAllByRole('tab').length > 0,
  hasTabpanelRole: (): boolean => screen.queryAllByRole('tabpanel', { hidden: true }).length > 0
}
