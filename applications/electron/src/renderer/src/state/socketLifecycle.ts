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
  resetBackoff(): void
}

type LifecycleDeps = {
  dispatch(action: StateAction): void
  onFrame(raw: string): void
  identify(): ClientMessage
}

export const socketLifecycle = ({
  dispatch,
  onFrame,
  identify
}: LifecycleDeps): SocketLifecycle => {
  let socket: WebSocket | null = null
  let attempt: Result.Async<WebSocket, unknown> | null = null
  let backoff = INITIAL_BACKOFF_MS
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let awaitingReconnect = false

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

  const wire = (sock: WebSocket, url: string): void => {
    socket = sock
    backoff = INITIAL_BACKOFF_MS
    if (awaitingReconnect) {
      awaitingReconnect = false
      dispatch(reconnected())
    } else {
      dispatch(connected())
    }
    send(identify())
    sock.onmessage = (ev) => {
      if (socket !== sock) return
      if (typeof ev.data === 'string') onFrame(ev.data)
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
      dispatch(disconnected())
      scheduleReconnect(url)
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
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    const sock = socket
    socket = null
    if (sock) {
      sock.onmessage = null
      sock.onerror = null
      sock.onclose = null
    }
    const chain = attempt
    attempt = null
    chain?.cancel()
  }

  const resetBackoff = (): void => {
    backoff = INITIAL_BACKOFF_MS
    awaitingReconnect = false
  }

  return { send, connect, teardown, resetBackoff }
}
