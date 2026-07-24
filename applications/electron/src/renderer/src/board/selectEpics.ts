import type { Epic } from './types'

export type EpicsState = {
  epics: Epic[]
}

export const selectEpics = (state: EpicsState): Epic[] => state.epics
