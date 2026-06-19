import { dispatchSession } from './store'
import {
  boardHydrateRequested,
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
  type SessionConfig
} from './actions'
import { nextOlderHistoryRequest } from './olderHistory'
import type { HistoryMessageWire } from '../types'
import { startSentFrameCapture } from './testCapture'
import type { ClientMessage } from '../types'

export { getSessionSnapshot, subscribeSession, resetSessionForTest } from './store'
export {
  useConnectionStatus,
  useSessionId,
  useLiveMessages,
  useRoster,
  useReactionsByChannel,
  useCiSnapshot,
  useSyncStatus,
  useBoardSnapshot,
  useBoardInvalidateTick,
  useComments,
  useTimelineEvents
} from './hooks'
export { subscribeLiveMessages, subscribePtyOutput, subscribePtyExit } from './emitter'
export { encodeKeystroke, decodePtyData } from './ptyCodec'
export { RENDERER_IDENTITY } from './constants'
export type { SessionSnapshot } from './reducer'
export type { SendOptions, SessionConfig } from './actions'

export const startSession = (config: SessionConfig): void => dispatchSession(startRequested(config))

export const sendChat = (text: string, opts?: SendOptions): void =>
  dispatchSession(chatSendRequested(text, opts))

export const toggleReaction = (
  channel: string,
  messageKey: string,
  emoji: string,
  user: string
): void => dispatchSession(reactionToggleRequested(channel, messageKey, emoji, user))

export const requestCommentsHydrate = (storyNumber: number): void =>
  dispatchSession(commentsHydrateRequested(storyNumber))

export const requestTimelineHydrate = (storyNumber: number): void =>
  dispatchSession(timelineHydrateRequested(storyNumber))

export const requestOlderHistory = (
  before: string,
  limit: number
): Promise<HistoryMessageWire[]> => {
  const { requestId, promise } = nextOlderHistoryRequest()
  dispatchSession(olderHistoryRequested(requestId, before, limit))
  return promise
}

export const requestBoardHydrate = (): void => dispatchSession(boardHydrateRequested())

export const sendPtySubscribe = (terminalId: string): void =>
  dispatchSession(ptySubscribeRequested(terminalId))

export const sendPtyInput = (terminalId: string, keystroke: string): void =>
  dispatchSession(ptyInputRequested(terminalId, keystroke))

export const sendPtyResize = (terminalId: string, cols: number, rows: number): void =>
  dispatchSession(ptyResizeRequested(terminalId, cols, rows))

export const deliverServerFrameForTest = (raw: string): void => dispatchSession(frameReceived(raw))

export const captureSentFramesForTest = (): ClientMessage[] => startSentFrameCapture()
