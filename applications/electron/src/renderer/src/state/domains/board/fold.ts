import { orderByIds, removeById, upsertById } from '@components/board/storyFolds'
import { StateActionType, type StateAction } from '../../actions'
import type { BoardColumn, Epic, Story } from '@components/board/types'
import type { RoadmapCard, RoadmapOutcomeData } from '@components/board/roadmap/selectRoadmap'
import type { BoardIterationData } from '@components/board/iteration/selectIterations'

export type BoardSlice = {
  stories: Story[]
  columns: BoardColumn[]
  epics: Epic[]
  outcomes: RoadmapOutcomeData[]
  iterations: BoardIterationData[]
  roadmap: RoadmapCard[]
  boardInvalidateTick: number
}

export const emptyBoard = (): BoardSlice => ({
  stories: [],
  columns: [],
  epics: [],
  outcomes: [],
  iterations: [],
  roadmap: [],
  boardInvalidateTick: 0
})

export const boardFold = (slice: BoardSlice, action: StateAction): BoardSlice | null => {
  if (action.type !== StateActionType.MirrorDeltaReceived) return null
  const delta = action.delta
  switch (delta.domain) {
    case 'stories':
      return {
        ...slice,
        stories: delta.stories,
        columns: delta.columns !== undefined ? delta.columns : slice.columns
      }
    case 'story_upserted':
      return { ...slice, stories: upsertById(slice.stories, delta.story) }
    case 'story_removed':
      return { ...slice, stories: removeById(slice.stories, delta.id) }
    case 'story_order':
      return { ...slice, stories: orderByIds(slice.stories, delta.ids) }
    case 'epics':
      return { ...slice, epics: delta.epics }
    case 'epic_upserted':
      return { ...slice, epics: upsertById(slice.epics, delta.epic) }
    case 'epic_removed':
      return { ...slice, epics: removeById(slice.epics, delta.id) }
    case 'epic_order':
      return { ...slice, epics: orderByIds(slice.epics, delta.ids) }
    case 'outcomes':
      return { ...slice, outcomes: delta.outcomes }
    case 'outcome_upserted':
      return { ...slice, outcomes: upsertById(slice.outcomes, delta.outcome) }
    case 'outcome_removed':
      return { ...slice, outcomes: removeById(slice.outcomes, delta.id) }
    case 'outcome_order':
      return { ...slice, outcomes: orderByIds(slice.outcomes, delta.ids) }
    case 'iterations':
      return { ...slice, iterations: delta.iterations }
    case 'iteration_upserted':
      return { ...slice, iterations: upsertById(slice.iterations, delta.iteration) }
    case 'iteration_removed':
      return { ...slice, iterations: removeById(slice.iterations, delta.id) }
    case 'iteration_order':
      return { ...slice, iterations: orderByIds(slice.iterations, delta.ids) }
    case 'roadmap':
      return { ...slice, roadmap: delta.roadmap }
    case 'roadmap_card_upserted':
      return { ...slice, roadmap: upsertById(slice.roadmap, delta.card) }
    case 'roadmap_card_removed':
      return { ...slice, roadmap: removeById(slice.roadmap, delta.id) }
    case 'roadmap_order':
      return { ...slice, roadmap: orderByIds(slice.roadmap, delta.ids) }
    case 'board_invalidated':
      return { ...slice, boardInvalidateTick: slice.boardInvalidateTick + 1 }
    default:
      return null
  }
}
