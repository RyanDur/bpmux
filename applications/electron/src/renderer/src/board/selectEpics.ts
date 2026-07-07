import type { Epic } from './types'

export interface EpicsState {
  epics: Epic[]
}

export const selectEpics = (state: EpicsState): Epic[] => state.epics
