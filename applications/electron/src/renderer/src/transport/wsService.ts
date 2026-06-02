import { describeHttpError, http } from '../api/http'
import { asyncTryCatch } from '../lib/asyncResult'
import { tryCatch } from '../lib/result'
import { applyStateDelta, applyStateSnapshot, emptyMirrorState } from './state-delta'
import type { MirrorState } from './state-delta'
import {
  BOARD_INVALIDATE_MARKER,
  SYNC_STATUS_UPDATE_MARKER,
  type ChatMessage,
  type ClientMessage,
  type ConnectionStatus,
  type ServerMessage
} from './types'

export const RENDERER_IDENTITY = 'ryan'
const RENDERER_TOOL = 'electron'

const INITIAL_BACKOFF_MS = 1000
const MAX_BACKOFF_MS = 8000

const HISTORY_CHAT_LIMIT = 200
const HISTORY_ACTIVITY_LIMIT = 50

export interface SendOptions {
  addressee?: string | null
  to?: string[] | null
}

export interface WsServiceConfig {
  url: string | undefined
  apiBase?: string
  authToken?: string
}

export interface WsSnapshot {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  mirror: MirrorState
  boardInvalidateTick: number
  syncStatusTick: number
  reconnectTick: number
}

interface HistoryMessage {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

const emptySnapshot = (url: string | undefined): WsSnapshot => ({
  status: url ? 'connecting' : 'disconnected',
  messages: [],
  sessionId: null,
  mirror: emptyMirrorState(),
  boardInvalidateTick: 0,
  syncStatusTick: 0,
  reconnectTick: 0
})

interface ServiceState {
  config: WsServiceConfig | null
  snapshot: WsSnapshot
  socket: WebSocket | null
  backoff: number
  reconnectTimer: ReturnType<typeof setTimeout> | null
  msgCounter: number
  historyLoaded: boolean
  awaitingReconnect: boolean
  historyAbort: AbortController | null
  listeners: Set<() => void>
  liveMessageListeners: Set<(m: ChatMessage) => void>
  ptyOutputListeners: Map<string, Set<(data: string) => void>>
}

const freshState = (): ServiceState => ({
  config: null,
  snapshot: emptySnapshot(undefined),
  socket: null,
  backoff: INITIAL_BACKOFF_MS,
  reconnectTimer: null,
  msgCounter: 0,
  historyLoaded: false,
  awaitingReconnect: false,
  historyAbort: null,
  listeners: new Set(),
  liveMessageListeners: new Set(),
  ptyOutputListeners: new Map()
})

let state: ServiceState = freshState()

let sentFramesForTest: ClientMessage[] | null = null

const patch = (changes: Partial<WsSnapshot>): void => {
  state.snapshot = { ...state.snapshot, ...changes }
  for (const listener of state.listeners) listener()
}

const emitLiveMessage = (msg: ChatMessage): void => {
  for (const handler of state.liveMessageListeners) handler(msg)
}

const emitPtyOutput = (terminalId: string, data: string): void => {
  const handlers = state.ptyOutputListeners.get(terminalId)
  if (!handlers) return
  for (const handler of handlers) handler(data)
}

const sendJson = (msg: ClientMessage): void => {
  if (sentFramesForTest) sentFramesForTest.push(msg)
  const sock = state.socket
  if (!sock || sock.readyState !== WebSocket.OPEN) {
    console.error('bpmux ws: send dropped, socket not open', msg)
    return
  }
  sock.send(JSON.stringify(msg))
}

const handleServerMessage = (raw: string): void => {
  const parseResult = tryCatch(
    () => JSON.parse(raw) as ServerMessage,
    (err) => err
  )
  if (parseResult.kind === 'failure') {
    console.error('bpmux ws: non-JSON frame', parseResult.reason, raw)
    return
  }
  const parsed = parseResult.value
  switch (parsed.type) {
    case 'welcome':
      patch({ sessionId: parsed.session_id })
      break
    case 'message': {
      if (parsed.message_type === 'state-snapshot' && parsed.state_snapshot) {
        patch({ mirror: applyStateSnapshot(state.snapshot.mirror, parsed.state_snapshot) })
        break
      }
      if (parsed.message_type === 'state-delta' && parsed.state_delta) {
        patch({ mirror: applyStateDelta(state.snapshot.mirror, parsed.state_delta) })
        break
      }
      if (parsed.text === BOARD_INVALIDATE_MARKER) {
        patch({ boardInvalidateTick: state.snapshot.boardInvalidateTick + 1 })
        break
      }
      if (parsed.text === SYNC_STATUS_UPDATE_MARKER) {
        patch({ syncStatusTick: state.snapshot.syncStatusTick + 1 })
        break
      }
      state.msgCounter += 1
      const chat: ChatMessage = {
        id: `${parsed.ts}-${state.msgCounter}`,
        from: parsed.from,
        text: parsed.text,
        ts: parsed.ts,
        to: parsed.to ?? null,
        addressee: parsed.addressee ?? null,
        messageType: parsed.message_type ?? null
      }
      const key = messageKey(chat)
      if (state.snapshot.messages.some((m) => messageKey(m) === key)) break
      patch({ messages: [...state.snapshot.messages, chat] })
      emitLiveMessage(chat)
      break
    }
    case 'error':
      console.error('bpmux ws: server error', parsed.reason)
      break
    case 'pty_output':
      emitPtyOutput(parsed.terminal_id, parsed.data)
      break
    case 'pong':
      break
    default:
      console.error('bpmux ws: unknown frame', parsed)
  }
}

const mergeHistory = (combined: HistoryMessage[]): void => {
  const seen = new Set<string>()
  const merged: ChatMessage[] = []
  for (const h of combined) {
    const chat: ChatMessage = {
      id: `hist-${h.ts}-${merged.length}`,
      from: h.from,
      text: h.text,
      ts: h.ts,
      to: h.to ?? null,
      addressee: h.addressee ?? null,
      messageType: h.message_type ?? null
    }
    const k = messageKey(chat)
    if (seen.has(k)) continue
    seen.add(k)
    merged.push(chat)
  }
  for (const m of state.snapshot.messages) {
    const k = messageKey(m)
    if (seen.has(k)) continue
    seen.add(k)
    merged.push(m)
  }
  patch({ messages: merged })
}

const loadHistory = (apiBase: string, authToken: string): void => {
  state.historyLoaded = true
  const controller = new AbortController()
  state.historyAbort = controller
  const fetchHistoryWithRetry = async (endpoint: string): Promise<HistoryMessage[]> => {
    const delays = [0, 200, 500]
    let lastErr: unknown = null
    for (const delay of delays) {
      if (controller.signal.aborted) throw new Error('aborted')
      if (delay > 0) {
        await new Promise<void>((resolve) => setTimeout(resolve, delay))
        if (controller.signal.aborted) throw new Error('aborted')
      }
      const result = await http.get<HistoryMessage[]>(
        { apiBase, authToken },
        endpoint,
        controller.signal
      ).value
      if (result.kind === 'success') return result.value
      if (result.reason.kind === 'network' && result.reason.message.includes('abort')) {
        throw new Error('aborted')
      }
      lastErr = new Error(describeHttpError(result.reason))
    }
    throw lastErr instanceof Error ? lastErr : new Error(String(lastErr))
  }
  void asyncTryCatch(() =>
    Promise.all([
      fetchHistoryWithRetry(`/api/messages?chat_only=true&limit=${HISTORY_CHAT_LIMIT}`),
      fetchHistoryWithRetry(`/api/messages?message_type=activity&limit=${HISTORY_ACTIVITY_LIMIT}`)
    ])
  )
    .onSuccess(([chat, activity]) => {
      if (controller.signal.aborted) return
      mergeHistory([...chat, ...activity].sort((a, b) => a.ts.localeCompare(b.ts)))
    })
    .onFailure((err) => {
      if (err.name === 'AbortError' || err.message === 'aborted') return
      console.error('bpmux ws: history fetch failed', err)
      state.historyLoaded = false
    }).value
}

const scheduleReconnect = (url: string): void => {
  const delay = state.backoff
  state.backoff = Math.min(delay * 2, MAX_BACKOFF_MS)
  state.reconnectTimer = setTimeout(() => connect(url), delay)
}

const connect = (url: string): void => {
  patch({ status: 'connecting' })
  const constructResult = tryCatch(
    () => new WebSocket(url),
    (err) => err
  )
  if (constructResult.kind === 'failure') {
    console.error('bpmux ws: construct failed', constructResult.reason)
    scheduleReconnect(url)
    return
  }
  const sock = constructResult.value
  state.socket = sock

  sock.onopen = () => {
    if (state.socket !== sock) return
    state.backoff = INITIAL_BACKOFF_MS
    const recovering = state.awaitingReconnect
    if (recovering) {
      state.awaitingReconnect = false
      patch({ status: 'connected', reconnectTick: state.snapshot.reconnectTick + 1 })
    } else {
      patch({ status: 'connected' })
    }
    sendJson({ type: 'identify', name: RENDERER_IDENTITY, tool: RENDERER_TOOL })
    if (recovering && state.config?.apiBase && state.config?.authToken) {
      state.historyAbort?.abort()
      state.historyLoaded = false
      loadHistory(state.config.apiBase, state.config.authToken)
    }
  }

  sock.onmessage = (ev) => {
    if (state.socket !== sock) return
    if (typeof ev.data === 'string') handleServerMessage(ev.data)
  }

  sock.onerror = (ev) => {
    if (state.socket !== sock) return
    console.error('bpmux ws: socket error', ev)
  }

  sock.onclose = (ev) => {
    if (state.socket !== sock) return
    console.error('bpmux ws: closed', ev.code, ev.reason)
    state.socket = null
    state.awaitingReconnect = true
    patch({ status: 'disconnected' })
    scheduleReconnect(url)
  }
}

const teardownSocket = (): void => {
  if (state.reconnectTimer) {
    clearTimeout(state.reconnectTimer)
    state.reconnectTimer = null
  }
  state.historyAbort?.abort()
  const sock = state.socket
  state.socket = null
  if (sock) {
    sock.onopen = null
    sock.onmessage = null
    sock.onerror = null
    sock.onclose = null
    if (sock.readyState === WebSocket.OPEN || sock.readyState === WebSocket.CONNECTING) {
      sock.close(1000, 'renderer unmounting')
    }
  }
}

const sameConfig = (a: WsServiceConfig | null, b: WsServiceConfig): boolean =>
  a !== null && a.url === b.url && a.apiBase === b.apiBase && a.authToken === b.authToken

export const ensureWsStarted = (config: WsServiceConfig): void => {
  if (sameConfig(state.config, config)) return
  const wasRunning = state.config !== null
  if (wasRunning) {
    teardownSocket()
    state.backoff = INITIAL_BACKOFF_MS
    state.historyLoaded = false
    state.awaitingReconnect = false
  }
  state.config = config
  if (!config.url) {
    console.error('bpmux ws: no url provided via window.bpmux.wsUrl')
    patch({ status: 'disconnected' })
    return
  }
  if (!state.historyLoaded && config.apiBase && config.authToken) {
    loadHistory(config.apiBase, config.authToken)
  }
  connect(config.url)
}

export const sendChat = (text: string, opts?: SendOptions): void => {
  const trimmed = text.trim()
  if (!trimmed) return
  const frame: ClientMessage = { type: 'send', text: trimmed }
  if (opts?.addressee) frame.addressee = opts.addressee
  if (opts?.to && opts.to.length > 0) frame.to = opts.to
  sendJson(frame)
}

export const subscribeWs = (listener: () => void): (() => void) => {
  state.listeners.add(listener)
  return () => {
    state.listeners.delete(listener)
  }
}

export const subscribeWsLiveMessages = (handler: (m: ChatMessage) => void): (() => void) => {
  state.liveMessageListeners.add(handler)
  return () => {
    state.liveMessageListeners.delete(handler)
  }
}

export const subscribeWsPtyOutput = (
  terminalId: string,
  handler: (data: string) => void
): (() => void) => {
  let handlers = state.ptyOutputListeners.get(terminalId)
  if (!handlers) {
    handlers = new Set()
    state.ptyOutputListeners.set(terminalId, handlers)
  }
  handlers.add(handler)
  return () => {
    handlers.delete(handler)
  }
}

export const sendPtySubscribe = (terminalId: string): void => {
  sendJson({ type: 'pty_subscribe', terminal_id: terminalId })
}

export const encodeKeystroke = (data: string): string => {
  const bytes = new TextEncoder().encode(data)
  let binary = ''
  for (const b of bytes) binary += String.fromCharCode(b)
  return btoa(binary)
}

export const decodePtyData = (b64: string): Uint8Array => {
  const binary = atob(b64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

export const sendPtyInput = (terminalId: string, keystroke: string): void => {
  sendJson({ type: 'pty_input', terminal_id: terminalId, data: encodeKeystroke(keystroke) })
}

export const sendPtyResize = (terminalId: string, cols: number, rows: number): void => {
  sendJson({ type: 'pty_resize', terminal_id: terminalId, cols, rows })
}

export const getWsSnapshot = (): WsSnapshot => state.snapshot

export const __resetWsForTest = (): void => {
  teardownSocket()
  sentFramesForTest = null
  state = freshState()
}

export const __captureSentFramesForTest = (): ClientMessage[] => {
  sentFramesForTest = []
  return sentFramesForTest
}

export const __deliverServerFrameForTest = (raw: string): void => {
  handleServerMessage(raw)
}
