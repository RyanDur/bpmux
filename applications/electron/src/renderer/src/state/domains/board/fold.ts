import { orderByIds, removeById, upsertById } from '@components/board/storyFolds'
import { MirrorActionType } from '../mirror/actions'
import type { StateAction } from '../../actions'
import type { BoardColumn, Epic, Story } from '@components/board/types'
import type { RoadmapCard, RoadmapOutcomeData } from '@components/board/roadmap/selectRoadmap'
import type { BoardIterationData } from '@components/board/iteration/selectIterations'

export type BoardState = {
  stories: Story[]
  columns: BoardColumn[]
  epics: Epic[]
  outcomes: RoadmapOutcomeData[]
  iterations: BoardIterationData[]
  roadmap: RoadmapCard[]
  boardInvalidateTick: number
}

export const initialBoard: BoardState = {
  stories: [],
  columns: [],
  epics: [],
  outcomes: [],
  iterations: [],
  roadmap: [],
  boardInvalidateTick: 0
}

export const boardFold = (state: BoardState = initialBoard, action: StateAction): BoardState => {
  if (action.type !== MirrorActionType.MirrorDeltaReceived) return state
  const delta = action.delta
  switch (delta.domain) {
    case 'stories':
      return {
        ...state,
        stories: delta.stories,
        columns: delta.columns !== undefined ? delta.columns : state.columns
      }
    case 'story_upserted':
      return { ...state, stories: upsertById(state.stories, delta.story) }
    case 'story_removed':
      return { ...state, stories: removeById(state.stories, delta.id) }
    case 'story_order':
      return { ...state, stories: orderByIds(state.stories, delta.ids) }
    case 'epics':
      return { ...state, epics: delta.epics }
    case 'epic_upserted':
      return { ...state, epics: upsertById(state.epics, delta.epic) }
    case 'epic_removed':
      return { ...state, epics: removeById(state.epics, delta.id) }
    case 'epic_order':
      return { ...state, epics: orderByIds(state.epics, delta.ids) }
    case 'outcomes':
      return { ...state, outcomes: delta.outcomes }
    case 'outcome_upserted':
      return { ...state, outcomes: upsertById(state.outcomes, delta.outcome) }
    case 'outcome_removed':
      return { ...state, outcomes: removeById(state.outcomes, delta.id) }
    case 'outcome_order':
      return { ...state, outcomes: orderByIds(state.outcomes, delta.ids) }
    case 'iterations':
      return { ...state, iterations: delta.iterations }
    case 'iteration_upserted':
      return { ...state, iterations: upsertById(state.iterations, delta.iteration) }
    case 'iteration_removed':
      return { ...state, iterations: removeById(state.iterations, delta.id) }
    case 'iteration_order':
      return { ...state, iterations: orderByIds(state.iterations, delta.ids) }
    case 'roadmap':
      return { ...state, roadmap: delta.roadmap }
    case 'roadmap_card_upserted':
      return { ...state, roadmap: upsertById(state.roadmap, delta.card) }
    case 'roadmap_card_removed':
      return { ...state, roadmap: removeById(state.roadmap, delta.id) }
    case 'roadmap_order':
      return { ...state, roadmap: orderByIds(state.roadmap, delta.ids) }
    case 'board_invalidated':
      return { ...state, boardInvalidateTick: state.boardInvalidateTick + 1 }
    default:
      return state
  }
}
