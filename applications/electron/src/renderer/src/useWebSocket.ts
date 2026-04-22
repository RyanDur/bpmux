import { useCallback, useEffect, useRef, useState } from 'react'
import type { ChatMessage, ClientMessage, ConnectionStatus, ServerMessage } from './types'

// WebSocket hook for the bpmux session protocol.
//
// Protocol (see applications/bpmux/src/transport/ws.rs):
//   1. Client connects.
//   2. Client sends `{type:"identify", name, tool}` (5s server-side
//      timeout; anything else before identify draws an error reply).
//   3. Server replies `{type:"welcome", session_id}`, then streams
//      `{type:"message", ...}` frames for every broadcast.
//   4. Server emits WS-level Ping frames every 30s (browser responds
//      automatically — we don't handle those).
//   5. To send: `{type:"send", text}` for broadcast. `to` omitted =
//      broadcast. The server stamps `from` using our identify name.
//
// There is no bearer-token auth on the WS handshake — the token in
// `window.bpmux.authToken` gates the HTTP simulate_* endpoints, not
// the WS. Ryan's ownership model keeps the WS a loopback-only local
// channel with identify-by-name.

const RENDERER_IDENTITY = 'ryan'
const RENDERER_TOOL = 'electron'

const INITIAL_BACKOFF_MS = 1000
const MAX_BACKOFF_MS = 8000

export interface UseWebSocketResult {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  sendBroadcast: (text: string) => void
}

export function useWebSocket(url: string | undefined): UseWebSocketResult {
  // When no URL is provided (preload missing), start disconnected
  // rather than connecting — avoids a setState-in-effect cascade and
  // correctly represents the state.
  const [status, setStatus] = useState<ConnectionStatus>(url ? 'connecting' : 'disconnected')
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [sessionId, setSessionId] = useState<string | null>(null)

  const socketRef = useRef<WebSocket | null>(null)
  const backoffRef = useRef<number>(INITIAL_BACKOFF_MS)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const unmountedRef = useRef<boolean>(false)
  const msgCounterRef = useRef<number>(0)

  const sendJson = useCallback((msg: ClientMessage) => {
    const sock = socketRef.current
    if (!sock || sock.readyState !== WebSocket.OPEN) {
      console.error('bpmux ws: send dropped, socket not open', msg)
      return
    }
    sock.send(JSON.stringify(msg))
  }, [])

  const sendBroadcast = useCallback(
    (text: string) => {
      const trimmed = text.trim()
      if (!trimmed) return
      sendJson({ type: 'send', text: trimmed })
    },
    [sendJson]
  )

  const handleServerMessage = useCallback((raw: string) => {
    let parsed: ServerMessage
    try {
      parsed = JSON.parse(raw) as ServerMessage
    } catch (err) {
      console.error('bpmux ws: non-JSON frame', err, raw)
      return
    }
    switch (parsed.type) {
      case 'welcome':
        setSessionId(parsed.session_id)
        break
      case 'message': {
        msgCounterRef.current += 1
        const id = `${parsed.ts}-${msgCounterRef.current}`
        const chat: ChatMessage = {
          id,
          from: parsed.from,
          text: parsed.text,
          ts: parsed.ts,
          to: parsed.to ?? null,
          addressee: parsed.addressee ?? null,
          messageType: parsed.message_type ?? null
        }
        setMessages((prev) => [...prev, chat])
        break
      }
      case 'error':
        console.error('bpmux ws: server error', parsed.reason)
        break
      case 'pong':
        break
      default:
        console.error('bpmux ws: unknown frame', parsed)
    }
  }, [])

  useEffect(() => {
    if (!url) {
      console.error('bpmux ws: no url provided via window.bpmux.wsUrl')
      return
    }

    unmountedRef.current = false

    const connect = (): void => {
      if (unmountedRef.current) return
      setStatus('connecting')
      let sock: WebSocket
      try {
        sock = new WebSocket(url)
      } catch (err) {
        console.error('bpmux ws: construct failed', err)
        scheduleReconnect()
        return
      }
      socketRef.current = sock

      sock.onopen = () => {
        backoffRef.current = INITIAL_BACKOFF_MS
        setStatus('connected')
        sendJson({
          type: 'identify',
          name: RENDERER_IDENTITY,
          tool: RENDERER_TOOL
        })
      }

      sock.onmessage = (ev) => {
        if (typeof ev.data === 'string') {
          handleServerMessage(ev.data)
        }
      }

      sock.onerror = (ev) => {
        console.error('bpmux ws: socket error', ev)
      }

      sock.onclose = (ev) => {
        console.error('bpmux ws: closed', ev.code, ev.reason)
        socketRef.current = null
        setStatus('disconnected')
        scheduleReconnect()
      }
    }

    const scheduleReconnect = (): void => {
      if (unmountedRef.current) return
      const delay = backoffRef.current
      backoffRef.current = Math.min(delay * 2, MAX_BACKOFF_MS)
      reconnectTimerRef.current = setTimeout(connect, delay)
    }

    connect()

    return () => {
      unmountedRef.current = true
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current)
        reconnectTimerRef.current = null
      }
      const sock = socketRef.current
      socketRef.current = null
      if (sock && sock.readyState === WebSocket.OPEN) {
        sock.close(1000, 'renderer unmounting')
      } else if (sock) {
        sock.close()
      }
    }
  }, [url, sendJson, handleServerMessage])

  return { status, messages, sessionId, sendBroadcast }
}
