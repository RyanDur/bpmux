import { dispatchState } from './store'
import {
  chatSendRequested,
  commentsHydrateRequested,
  frameReceived,
  olderHistoryRequested,
  ptyInputRequested,
  ptyResizeRequested,
  ptySubscribeRequested,
  reactionToggleRequested,
  startRequested,
  timelineHydrateRequested,
  type SendOptions,
  type ConnectionConfig
} from './actions'
import { nextOlderHistoryRequest } from './olderHistory'
import type { HistoryMessageWire } from '../transport/types'
import { startSentFrameCapture } from './testCapture'
import type { ClientMessage } from '../transport/types'

export { getStateSnapshot, subscribeState, resetStateForTest } from './store'
export {
  useConnectionStatus,
  useLiveMessages,
  useRoster,
  useReactionsByChannel,
  useCiSnapshot,
  useSyncStatus,
  useStories,
  useBoardColumns,
  useBoardInvalidateTick,
  useComments,
  useTimelineEvents
} from './hooks'
export {
  subscribeLiveMessages,
  subscribePtyOutput,
  subscribePtyMode,
  subscribePtyExit
} from './emitter'
export { encodeKeystroke, decodePtyData } from './ptyCodec'
export { RENDERER_IDENTITY } from './constants'

export type { SendOptions, ConnectionConfig } from './actions'

export const startConnection = (config: ConnectionConfig): void =>
  dispatchState(startRequested(config))

export const sendChat = (text: string, opts?: SendOptions): void =>
  dispatchState(chatSendRequested(text, opts))

export const toggleReaction = (
  channel: string,
  messageKey: string,
  emoji: string,
  user: string
): void => dispatchState(reactionToggleRequested(channel, messageKey, emoji, user))

export const requestCommentsHydrate = (storyId: number): void =>
  dispatchState(commentsHydrateRequested(storyId))

export const requestTimelineHydrate = (storyId: number): void =>
  dispatchState(timelineHydrateRequested(storyId))

export const requestOlderHistory = (
  before: string,
  limit: number
): Promise<HistoryMessageWire[]> => {
  const { requestId, promise } = nextOlderHistoryRequest()
  dispatchState(olderHistoryRequested(requestId, before, limit))
  return promise
}

export const sendPtySubscribe = (terminalId: string): void =>
  dispatchState(ptySubscribeRequested(terminalId))

export const sendPtyInput = (terminalId: string, keystroke: string): void =>
  dispatchState(ptyInputRequested(terminalId, keystroke))

export const sendPtyResize = (terminalId: string, cols: number, rows: number): void =>
  dispatchState(ptyResizeRequested(terminalId, cols, rows))

export const deliverServerFrameForTest = (raw: string): void => dispatchState(frameReceived(raw))

export const captureSentFramesForTest = (): ClientMessage[] => startSentFrameCapture()
