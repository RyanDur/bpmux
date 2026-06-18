import { tryCatch } from '../../lib/result'
import type { Middleware } from '../../lib/redux'
import type { SessionSnapshot } from './reducer'
import {
  SessionActionType,
  connected,
  connecting,
  disconnected,
  reconnected,
  type SessionAction,
  type SessionConfig
} from './actions'
import { dispatchServerFrame } from './decode'
import { encodeKeystroke } from './ptyCodec'
import { INITIAL_BACKOFF_MS, MAX_BACKOFF_MS, RENDERER_IDENTITY, RENDERER_TOOL } from './constants'
import { captureSentFrame } from './testCapture'
import type { ClientMessage } from '../types'

const sameConfig = (a: SessionConfig | null, b: SessionConfig): boolean =>
  a !== null && a.url === b.url && a.apiBase === b.apiBase && a.authToken === b.authToken

export const socketMiddleware: Middleware<SessionSnapshot, SessionAction> = (api) => {
  let socket: WebSocket | null = null
  let backoff = INITIAL_BACKOFF_MS
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let awaitingReconnect = false
  let messageSeq = 0
  let config: SessionConfig | null = null

  const send = (msg: ClientMessage): void => {
    captureSentFrame(msg)
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      console.error('bpmux session: send dropped, socket not open', msg)
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
    if (constructResult.kind === 'failure') {
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
      send({ type: 'identify', name: RENDERER_IDENTITY, tool: RENDERER_TOOL })
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
      case SessionActionType.StartRequested: {
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
      case SessionActionType.ChatSendRequested: {
        const trimmed = action.text.trim()
        if (!trimmed) return
        const frame: ClientMessage = { type: 'send', text: trimmed }
        if (action.opts?.addressee) frame.addressee = action.opts.addressee
        if (action.opts?.to && action.opts.to.length > 0) frame.to = action.opts.to
        send(frame)
        return
      }
      case SessionActionType.ReactionToggleRequested:
        send({
          type: 'reaction_toggle',
          channel: action.channel,
          message_key: action.messageKey,
          emoji: action.emoji,
          user: action.user
        })
        return
      case SessionActionType.CommentsHydrateRequested:
        send({ type: 'comments_hydrate', story_number: action.storyNumber })
        return
      case SessionActionType.PtySubscribeRequested:
        send({ type: 'pty_subscribe', terminal_id: action.terminalId })
        return
      case SessionActionType.PtyInputRequested:
        send({
          type: 'pty_input',
          terminal_id: action.terminalId,
          data: encodeKeystroke(action.keystroke)
        })
        return
      case SessionActionType.PtyResizeRequested:
        send({
          type: 'pty_resize',
          terminal_id: action.terminalId,
          cols: action.cols,
          rows: action.rows
        })
        return
      case SessionActionType.FrameReceived:
        dispatchServerFrame(action.raw, api.dispatch, () => (messageSeq += 1))
        return
      case SessionActionType.Reset:
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
