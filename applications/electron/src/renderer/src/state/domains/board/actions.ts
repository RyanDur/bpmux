export enum BoardActionType {
  OutcomeHorizonChangeRequested = 'state/outcomeHorizonChangeRequested',
  OutcomeIterationReorderRequested = 'state/outcomeIterationReorderRequested',
  OutcomeReprioritizeRequested = 'state/outcomeReprioritizeRequested',
  IterationReorderRequested = 'state/iterationReorderRequested',
  AddStoryToIterationRequested = 'state/addStoryToIterationRequested'
}

export type BoardAction =
  | {
      type: BoardActionType.OutcomeHorizonChangeRequested
      outcomeId: number
      horizon: 'now' | 'next' | 'later'
    }
  | {
      type: BoardActionType.OutcomeIterationReorderRequested
      outcomeId: number
      iterationId: number
      position: number
    }
  | { type: BoardActionType.OutcomeReprioritizeRequested; outcomeId: number; position: number }
  | { type: BoardActionType.IterationReorderRequested; iterationId: number; position: number }
  | {
      type: BoardActionType.AddStoryToIterationRequested
      iterationId: number
      storyId: number
      position: number
    }

export const outcomeHorizonChangeRequested = (
  outcomeId: number,
  horizon: 'now' | 'next' | 'later'
): BoardAction => ({
  type: BoardActionType.OutcomeHorizonChangeRequested,
  outcomeId,
  horizon
})

export const outcomeIterationReorderRequested = (
  outcomeId: number,
  iterationId: number,
  position: number
): BoardAction => ({
  type: BoardActionType.OutcomeIterationReorderRequested,
  outcomeId,
  iterationId,
  position
})

export const outcomeReprioritizeRequested = (outcomeId: number, position: number): BoardAction => ({
  type: BoardActionType.OutcomeReprioritizeRequested,
  outcomeId,
  position
})

export const iterationReorderRequested = (iterationId: number, position: number): BoardAction => ({
  type: BoardActionType.IterationReorderRequested,
  iterationId,
  position
})

export const addStoryToIterationRequested = (
  iterationId: number,
  storyId: number,
  position: number
): BoardAction => ({
  type: BoardActionType.AddStoryToIterationRequested,
  iterationId,
  storyId,
  position
})
