import { type ConnectionSlice, connectionFold, emptyConnection } from './domains/connection/fold'
import { type ChatSlice, chatFold, emptyChat } from './domains/chat/fold'
import { type DrillInSlice, drillInFold, emptyDrillIn } from './domains/drillIn/fold'
import { type BoardSlice, boardFold, emptyBoard } from './domains/board/fold'
import { type MirrorState, emptyMirrorState, mirrorFold } from './domains/mirror/fold'
import type { StateAction } from './actions'

export type StateSnapshot = ConnectionSlice &
  ChatSlice &
  DrillInSlice &
  BoardSlice & { mirror: MirrorState }

export const emptySnapshot = (url: string | undefined): StateSnapshot => ({
  ...emptyConnection(url),
  ...emptyChat(),
  ...emptyDrillIn(),
  ...emptyBoard(),
  mirror: emptyMirrorState()
})

export const stateReducer = (state: StateSnapshot, action: StateAction): StateSnapshot => {
  const patches = [
    connectionFold(state, action),
    chatFold(state, action),
    drillInFold(state, action),
    boardFold(state, action),
    mirrorFold(state, action)
  ].filter((patch) => patch !== null)
  return patches.length === 0 ? state : Object.assign({}, state, ...patches)
}
