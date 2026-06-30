import type { Story } from './types'

export interface EpicsState {
  epics: Story[]
}

export const selectEpics = (state: EpicsState): Story[] => state.epics
