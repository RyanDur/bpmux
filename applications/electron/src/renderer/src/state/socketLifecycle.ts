import { type Result, connecting } from '@ryandur/sand'
import {
  type StateAction,
  connected,
  connecting as connectingStarted,
  disconnected,
  reconnected
} from './actions'
import { rejectPendingOlderHistory } from './olderHistory'
import { INITIAL_BACKOFF_MS, MAX_BACKOFF_MS } from './constants'
import { captureSentFrame } from './testCapture'
import type { ClientMessage } from '@transport/types'

export type SocketLifecycle = {
  send(msg: ClientMessage): void
  connect(url: string): void
  teardown(): void
}

type LifecycleDeps = {
  dispatch(action: StateAction): void
  onFrame(raw: string): void
  identify(): ClientMessage
}

export const socketLifecycle = (deps: LifecycleDeps): SocketLifecycle => {
  const { dispatch, onFrame, identify } = deps
  let socket: WebSocket | null = null
  let attempt: Result.Async<WebSocket, unknown> | null = null
  let backoff = INITIAL_BACKOFF_MS
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let awaitingReconnect = false

  const send = (msg: ClientMessage): void => {
    captureSentFrame(msg)
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      console.warn('bpmux ws: send dropped, socket not open', msg)
      return
    }
    socket.send(JSON.stringify(msg))
  }

  const scheduleReconnect = (url: string): void => {
    reconnectTimer = setTimeout(() => connect(url), backoff)
    backoff = Math.min(backoff * 2, MAX_BACKOFF_MS)
  }

  const announce = (): void => {
    backoff = INITIAL_BACKOFF_MS
    dispatch(awaitingReconnect ? reconnected() : connected())
    awaitingReconnect = false
    send(identify())
  }

  const dropped = (url: string, ev: CloseEvent): void => {
    console.warn('bpmux ws: closed, reconnecting', ev.code, ev.reason)
    socket = null
    awaitingReconnect = true
    rejectPendingOlderHistory('socket closed')
    dispatch(disconnected())
    scheduleReconnect(url)
  }

  const wire = (sock: WebSocket, url: string): void => {
    socket = sock
    announce()
    sock.onmessage = (ev) => {
      if (socket === sock && typeof ev.data === 'string') onFrame(ev.data)
    }
    sock.onclose = (ev) => {
      if (socket === sock) dropped(url, ev)
    }
  }

  const connect = (url: string): void => {
    dispatch(connectingStarted())
    const chain = connecting(url, (thrown) => thrown)
    attempt = chain
    chain
      .onSuccess((sock) => wire(sock, url))
      .onFailure((thrown) => {
        console.warn('bpmux ws: handshake failed, retrying', thrown)
        scheduleReconnect(url)
      })
  }

  const teardown = (): void => {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    reconnectTimer = null
    if (socket) socket.onmessage = socket.onerror = socket.onclose = null
    socket = null
    attempt?.cancel()
    attempt = null
    backoff = INITIAL_BACKOFF_MS
    awaitingReconnect = false
  }

  return { send, connect, teardown }
}
