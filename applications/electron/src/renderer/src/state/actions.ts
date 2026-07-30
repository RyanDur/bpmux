import { ConnectionActionType, type ConnectionAction } from './domains/connection/actions'
import { SocketActionType, type SocketAction } from './socket/actions'
import { ChatActionType, type ChatAction } from './domains/chat/actions'
import { DrillInActionType, type DrillInAction } from './domains/drillIn/actions'
import { BoardActionType, type BoardAction } from './domains/board/actions'
import { MirrorActionType, type MirrorAction } from './domains/mirror/actions'
import { PtyActionType, type PtyAction } from './domains/pty/actions'

export { connected, connecting, disconnected, reconnected } from './domains/connection/actions'
export { frameReceived, reset, startRequested, type ConnectionConfig } from './socket/actions'
export {
  chatSendRequested,
  historyLoaded,
  messageReceived,
  olderHistoryRequested,
  reactionToggleRequested,
  type SendOptions
} from './domains/chat/actions'
export {
  commentsHydrateRequested,
  commentsHydrated,
  timelineHydrateRequested,
  timelineHydrated
} from './domains/drillIn/actions'
export {
  addStoryToIterationRequested,
  iterationReorderRequested,
  outcomeHorizonChangeRequested,
  outcomeIterationReorderRequested,
  outcomeReprioritizeRequested
} from './domains/board/actions'
export { mirrorDeltaReceived } from './domains/mirror/actions'
export {
  ptyExitReceived,
  ptyInputRequested,
  ptyModeReceived,
  ptyOutputReceived,
  ptyResizeRequested,
  ptySubscribeRequested
} from './domains/pty/actions'

export type StateAction =
  | ConnectionAction
  | SocketAction
  | ChatAction
  | DrillInAction
  | BoardAction
  | MirrorAction
  | PtyAction

export const StateActionType = {
  ...ConnectionActionType,
  ...SocketActionType,
  ...ChatActionType,
  ...DrillInActionType,
  ...BoardActionType,
  ...MirrorActionType,
  ...PtyActionType
} as const
