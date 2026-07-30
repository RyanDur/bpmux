import { caseOf } from '@ryandur/sand'
import { orderByIds, removeById, upsertById } from '@components/board/storyFolds'
import { StateActionType, type StateAction } from '../../actions'
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

const onActionType = caseOf('type')
const onDomain = caseOf('domain')

export const boardFold = (state: BoardState = initialBoard, action: StateAction): BoardState =>
  onActionType<StateAction, BoardState>(action, {
    [StateActionType.MirrorDeltaReceived]: ({ delta }) =>
      onDomain<typeof delta, BoardState>(delta, {
        stories: ({ stories, columns }) => ({
          ...state,
          stories,
          columns: columns !== undefined ? columns : state.columns
        }),
        story_upserted: ({ story }) => ({ ...state, stories: upsertById(state.stories, story) }),
        story_removed: ({ id }) => ({ ...state, stories: removeById(state.stories, id) }),
        story_order: ({ ids }) => ({ ...state, stories: orderByIds(state.stories, ids) }),
        epics: ({ epics }) => ({ ...state, epics }),
        epic_upserted: ({ epic }) => ({ ...state, epics: upsertById(state.epics, epic) }),
        epic_removed: ({ id }) => ({ ...state, epics: removeById(state.epics, id) }),
        epic_order: ({ ids }) => ({ ...state, epics: orderByIds(state.epics, ids) }),
        outcomes: ({ outcomes }) => ({ ...state, outcomes }),
        outcome_upserted: ({ outcome }) => ({
          ...state,
          outcomes: upsertById(state.outcomes, outcome)
        }),
        outcome_removed: ({ id }) => ({ ...state, outcomes: removeById(state.outcomes, id) }),
        outcome_order: ({ ids }) => ({ ...state, outcomes: orderByIds(state.outcomes, ids) }),
        iterations: ({ iterations }) => ({ ...state, iterations }),
        iteration_upserted: ({ iteration }) => ({
          ...state,
          iterations: upsertById(state.iterations, iteration)
        }),
        iteration_removed: ({ id }) => ({ ...state, iterations: removeById(state.iterations, id) }),
        iteration_order: ({ ids }) => ({ ...state, iterations: orderByIds(state.iterations, ids) }),
        roadmap: ({ roadmap }) => ({ ...state, roadmap }),
        roadmap_card_upserted: ({ card }) => ({
          ...state,
          roadmap: upsertById(state.roadmap, card)
        }),
        roadmap_card_removed: ({ id }) => ({ ...state, roadmap: removeById(state.roadmap, id) }),
        roadmap_order: ({ ids }) => ({ ...state, roadmap: orderByIds(state.roadmap, ids) }),
        board_invalidated: () => ({
          ...state,
          boardInvalidateTick: state.boardInvalidateTick + 1
        })
      }).orElse(state)
  }).orElse(state)
