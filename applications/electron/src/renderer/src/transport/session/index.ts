import { dispatchSession } from './store'
import {
  chatSendRequested,
  frameReceived,
  ptyInputRequested,
  ptyResizeRequested,
  ptySubscribeRequested,
  startRequested,
  type SendOptions,
  type SessionConfig
} from './actions'
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
  useBoardInvalidateTick
} from './hooks'
export { subscribeLiveMessages, subscribePtyOutput, subscribePtyExit } from './emitter'
export { encodeKeystroke, decodePtyData } from './ptyCodec'
export { RENDERER_IDENTITY } from './constants'
export type { SessionSnapshot } from './reducer'
export type { SendOptions, SessionConfig } from './actions'

export const startSession = (config: SessionConfig): void => dispatchSession(startRequested(config))

export const sendChat = (text: string, opts?: SendOptions): void =>
  dispatchSession(chatSendRequested(text, opts))

export const sendPtySubscribe = (terminalId: string): void =>
  dispatchSession(ptySubscribeRequested(terminalId))

export const sendPtyInput = (terminalId: string, keystroke: string): void =>
  dispatchSession(ptyInputRequested(terminalId, keystroke))

export const sendPtyResize = (terminalId: string, cols: number, rows: number): void =>
  dispatchSession(ptyResizeRequested(terminalId, cols, rows))

export const deliverServerFrameForTest = (raw: string): void => dispatchSession(frameReceived(raw))

export const captureSentFramesForTest = (): ClientMessage[] => startSentFrameCapture()
