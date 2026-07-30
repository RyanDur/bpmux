import { caseOf } from '@ryandur/sand'
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

const onActionType = caseOf('type')
const onDomain = caseOf('domain')

export const boardFold = (slice: BoardSlice, action: StateAction): BoardSlice | null =>
  onActionType<StateAction, BoardSlice | null>(action, {
    [StateActionType.MirrorDeltaReceived]: ({ delta }) =>
      onDomain<typeof delta, BoardSlice>(delta, {
        stories: ({ stories, columns }) => ({
          ...slice,
          stories,
          columns: columns !== undefined ? columns : slice.columns
        }),
        story_upserted: ({ story }) => ({ ...slice, stories: upsertById(slice.stories, story) }),
        story_removed: ({ id }) => ({ ...slice, stories: removeById(slice.stories, id) }),
        story_order: ({ ids }) => ({ ...slice, stories: orderByIds(slice.stories, ids) }),
        epics: ({ epics }) => ({ ...slice, epics }),
        epic_upserted: ({ epic }) => ({ ...slice, epics: upsertById(slice.epics, epic) }),
        epic_removed: ({ id }) => ({ ...slice, epics: removeById(slice.epics, id) }),
        epic_order: ({ ids }) => ({ ...slice, epics: orderByIds(slice.epics, ids) }),
        outcomes: ({ outcomes }) => ({ ...slice, outcomes }),
        outcome_upserted: ({ outcome }) => ({
          ...slice,
          outcomes: upsertById(slice.outcomes, outcome)
        }),
        outcome_removed: ({ id }) => ({ ...slice, outcomes: removeById(slice.outcomes, id) }),
        outcome_order: ({ ids }) => ({ ...slice, outcomes: orderByIds(slice.outcomes, ids) }),
        iterations: ({ iterations }) => ({ ...slice, iterations }),
        iteration_upserted: ({ iteration }) => ({
          ...slice,
          iterations: upsertById(slice.iterations, iteration)
        }),
        iteration_removed: ({ id }) => ({ ...slice, iterations: removeById(slice.iterations, id) }),
        iteration_order: ({ ids }) => ({ ...slice, iterations: orderByIds(slice.iterations, ids) }),
        roadmap: ({ roadmap }) => ({ ...slice, roadmap }),
        roadmap_card_upserted: ({ card }) => ({
          ...slice,
          roadmap: upsertById(slice.roadmap, card)
        }),
        roadmap_card_removed: ({ id }) => ({ ...slice, roadmap: removeById(slice.roadmap, id) }),
        roadmap_order: ({ ids }) => ({ ...slice, roadmap: orderByIds(slice.roadmap, ids) }),
        board_invalidated: () => ({
          ...slice,
          boardInvalidateTick: slice.boardInvalidateTick + 1
        })
      }).orNull()
  }).orNull()
