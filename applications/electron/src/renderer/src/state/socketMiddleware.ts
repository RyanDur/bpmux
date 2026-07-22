import { tryCatch } from '@ryandur/sand'
import type { Middleware } from './redux'
import type { StateSnapshot } from './reducer'
import {
  StateActionType,
  connected,
  connecting,
  disconnected,
  reconnected,
  type StateAction,
  type ConnectionConfig
} from './actions'
import { dispatchServerFrame } from './decode'
import { rejectPendingOlderHistory } from './olderHistory'
import { encodeKeystroke } from './ptyCodec'
import { INITIAL_BACKOFF_MS, MAX_BACKOFF_MS, RENDERER_IDENTITY, RENDERER_TOOL } from './constants'
import { captureSentFrame } from './testCapture'
import type { ClientMessage } from '../transport/types'

const sameConfig = (a: ConnectionConfig | null, b: ConnectionConfig): boolean =>
  a !== null &&
  a.url === b.url &&
  a.apiBase === b.apiBase &&
  a.authToken === b.authToken &&
  a.ownerNonce === b.ownerNonce

export const socketMiddleware: Middleware<StateSnapshot, StateAction> = (api) => {
  let socket: WebSocket | null = null
  let backoff = INITIAL_BACKOFF_MS
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let awaitingReconnect = false
  let messageSeq = 0
  let config: ConnectionConfig | null = null

  const send = (msg: ClientMessage): void => {
    captureSentFrame(msg)
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      console.warn('bpmux state: send dropped, socket not open', msg)
      return
    }
    socket.send(JSON.stringify(msg))
  }

  const scheduleReconnect = (url: string): void => {
    const delay = backoff
    backoff = Math.min(delay * 2, MAX_BACKOFF_MS)
    reconnectTimer = setTimeout(() => connect(url), delay)
  }

  const connect = (url: string): void => {
    api.dispatch(connecting())
    const constructResult = tryCatch(
      () => new WebSocket(url),
      (err) => err
    )
    if (!constructResult.isSuccess) {
      console.warn('bpmux ws: construct failed, retrying', constructResult.reason)
      scheduleReconnect(url)
      return
    }
    const sock = constructResult.value
    socket = sock
    sock.onopen = () => {
      if (socket !== sock) return
      backoff = INITIAL_BACKOFF_MS
      if (awaitingReconnect) {
        awaitingReconnect = false
        api.dispatch(reconnected())
      } else {
        api.dispatch(connected())
      }
      send({
        type: 'identify',
        name: RENDERER_IDENTITY,
        tool: RENDERER_TOOL,
        ...(config?.ownerNonce ? { owner_nonce: config.ownerNonce } : {})
      })
    }
    sock.onmessage = (ev) => {
      if (socket !== sock) return
      if (typeof ev.data === 'string') {
        dispatchServerFrame(ev.data, api.dispatch, () => (messageSeq += 1))
      }
    }
    sock.onerror = (ev) => {
      if (socket !== sock) return
      console.warn('bpmux ws: socket error', ev)
    }
    sock.onclose = (ev) => {
      if (socket !== sock) return
      console.warn('bpmux ws: closed, reconnecting', ev.code, ev.reason)
      socket = null
      awaitingReconnect = true
      rejectPendingOlderHistory('socket closed')
      api.dispatch(disconnected())
      scheduleReconnect(url)
    }
  }

  const teardown = (): void => {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    const sock = socket
    socket = null
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

  return (next) => (action) => {
    switch (action.type) {
      case StateActionType.StartRequested: {
        if (sameConfig(config, action.config)) return
        if (config !== null) {
          teardown()
          backoff = INITIAL_BACKOFF_MS
          awaitingReconnect = false
        }
        config = action.config
        next(action)
        if (!action.config.url) {
          console.error('bpmux ws: no url provided via window.bpmux.wsUrl')
          api.dispatch(disconnected())
          return
        }
        connect(action.config.url)
        return
      }
      case StateActionType.ChatSendRequested: {
        const trimmed = action.text.trim()
        if (!trimmed) return
        const frame: ClientMessage = { type: 'send', text: trimmed }
        if (action.opts?.addressee) frame.addressee = action.opts.addressee
        if (action.opts?.to && action.opts.to.length > 0) frame.to = action.opts.to
        send(frame)
        return
      }
      case StateActionType.ReactionToggleRequested:
        send({
          type: 'reaction_toggle',
          channel: action.channel,
          message_key: action.messageKey,
          emoji: action.emoji,
          user: action.user
        })
        return
      case StateActionType.CommentsHydrateRequested:
        send({ type: 'comments_hydrate', story_id: action.storyId })
        return
      case StateActionType.TimelineHydrateRequested:
        send({ type: 'timeline_hydrate', story_id: action.storyId })
        return
      case StateActionType.HistoryHydrateRequested:
        send({
          type: 'history_hydrate',
          chat_limit: action.chatLimit,
          activity_limit: action.activityLimit
        })
        return
      case StateActionType.OlderHistoryRequested:
        send({
          type: 'history_page',
          request_id: action.requestId,
          before: action.before,
          limit: action.limit
        })
        return
      case StateActionType.OutcomeHorizonChangeRequested:
        send({
          type: 'set_outcome_horizon',
          outcome_id: action.outcomeId,
          horizon: action.horizon
        })
        return
      case StateActionType.OutcomeIterationReorderRequested:
        send({
          type: 'reorder_outcome_iteration',
          outcome_id: action.outcomeId,
          iteration_id: action.iterationId,
          position: action.position
        })
        return
      case StateActionType.OutcomeReprioritizeRequested:
        send({
          type: 'reprioritize_outcome',
          outcome_id: action.outcomeId,
          position: action.position
        })
        return
      case StateActionType.IterationReorderRequested:
        send({
          type: 'reorder_iteration',
          iteration_id: action.iterationId,
          position: action.position
        })
        return
      case StateActionType.AddStoryToIterationRequested:
        send({
          type: 'add_story_to_iteration',
          iteration_id: action.iterationId,
          story_id: action.storyId,
          position: action.position
        })
        return
      case StateActionType.PtySubscribeRequested:
        send({ type: 'pty_subscribe', terminal_id: action.terminalId })
        return
      case StateActionType.PtyInputRequested:
        send({
          type: 'pty_input',
          terminal_id: action.terminalId,
          data: encodeKeystroke(action.keystroke)
        })
        return
      case StateActionType.PtyResizeRequested:
        send({
          type: 'pty_resize',
          terminal_id: action.terminalId,
          cols: action.cols,
          rows: action.rows
        })
        return
      case StateActionType.FrameReceived:
        dispatchServerFrame(action.raw, api.dispatch, () => (messageSeq += 1))
        return
      case StateActionType.Reset:
        teardown()
        backoff = INITIAL_BACKOFF_MS
        awaitingReconnect = false
        messageSeq = 0
        config = null
        next(action)
        return
      default:
        next(action)
    }
  }
}
