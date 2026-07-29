import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import { StateActionType, type StateAction } from '../actions'
import type { SocketDoor } from './door'

export const outcomeHorizon =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.OutcomeHorizonChangeRequested) return next(action)
    door.send({
      type: 'set_outcome_horizon',
      outcome_id: action.outcomeId,
      horizon: action.horizon
    })
  }

export const outcomeIterationReorder =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.OutcomeIterationReorderRequested) return next(action)
    door.send({
      type: 'reorder_outcome_iteration',
      outcome_id: action.outcomeId,
      iteration_id: action.iterationId,
      position: action.position
    })
  }

export const outcomeReprioritize =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.OutcomeReprioritizeRequested) return next(action)
    door.send({
      type: 'reprioritize_outcome',
      outcome_id: action.outcomeId,
      position: action.position
    })
  }

export const iterationReorder =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.IterationReorderRequested) return next(action)
    door.send({
      type: 'reorder_iteration',
      iteration_id: action.iterationId,
      position: action.position
    })
  }

export const addStoryToIteration =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.AddStoryToIterationRequested) return next(action)
    door.send({
      type: 'add_story_to_iteration',
      iteration_id: action.iterationId,
      story_id: action.storyId,
      position: action.position
    })
  }
