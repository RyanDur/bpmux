import { StateActionType } from '../actions'
import type { Translator } from './translating'

export const outcomeHorizon: Translator = (action) =>
  action.type === StateActionType.OutcomeHorizonChangeRequested
    ? { type: 'set_outcome_horizon', outcome_id: action.outcomeId, horizon: action.horizon }
    : null

export const outcomeIterationReorder: Translator = (action) =>
  action.type === StateActionType.OutcomeIterationReorderRequested
    ? {
        type: 'reorder_outcome_iteration',
        outcome_id: action.outcomeId,
        iteration_id: action.iterationId,
        position: action.position
      }
    : null

export const outcomeReprioritize: Translator = (action) =>
  action.type === StateActionType.OutcomeReprioritizeRequested
    ? { type: 'reprioritize_outcome', outcome_id: action.outcomeId, position: action.position }
    : null

export const iterationReorder: Translator = (action) =>
  action.type === StateActionType.IterationReorderRequested
    ? { type: 'reorder_iteration', iteration_id: action.iterationId, position: action.position }
    : null

export const addStoryToIteration: Translator = (action) =>
  action.type === StateActionType.AddStoryToIterationRequested
    ? {
        type: 'add_story_to_iteration',
        iteration_id: action.iterationId,
        story_id: action.storyId,
        position: action.position
      }
    : null
