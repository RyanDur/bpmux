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

export const RENDERER_IDENTITY = 'ryan'
const RENDERER_TOOL = 'electron'

const INITIAL_BACKOFF_MS = 1000
const MAX_BACKOFF_MS = 8000

export interface SendOptions {
  addressee?: string | null
  to?: string[] | null
}

export interface UseWebSocketResult {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  sendChat: (text: string, opts?: SendOptions) => void
}

interface HistoryMessage {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

function messageKey(m: { from: string; text: string; ts: string }): string {
  return `${m.ts}|${m.from}|${m.text}`
}

export function useWebSocket(
  url: string | undefined,
  apiBase?: string,
  authToken?: string
): UseWebSocketResult {
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
  const historyLoadedRef = useRef<boolean>(false)

  const sendJson = useCallback((msg: ClientMessage) => {
    const sock = socketRef.current
    if (!sock || sock.readyState !== WebSocket.OPEN) {
      console.error('bpmux ws: send dropped, socket not open', msg)
      return
    }
    sock.send(JSON.stringify(msg))
  }, [])

  const sendChat = useCallback(
    (text: string, opts?: SendOptions) => {
      const trimmed = text.trim()
      if (!trimmed) return
      const frame: ClientMessage = { type: 'send', text: trimmed }
      if (opts?.addressee) frame.addressee = opts.addressee
      if (opts?.to && opts.to.length > 0) frame.to = opts.to
      sendJson(frame)
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
        const key = messageKey(chat)
        setMessages((prev) => {
          if (prev.some((m) => messageKey(m) === key)) return prev
          return [...prev, chat]
        })
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

    if (!historyLoadedRef.current && apiBase && authToken) {
      historyLoadedRef.current = true
      const controller = new AbortController()
      const headers = { Authorization: `Bearer ${authToken}` }
      // Two parallel hydrations (#364):
      //   1. chat_only=true — regular chat + insight history for the
      //      MessageList; unchanged behavior.
      //   2. message_type=activity — small activity seed so the
      //      sidebar presence dots derive sensibly on first render.
      //      Without this, usePresence saw zero activity messages in
      //      the initial state (chat_only=true filters them out) and
      //      every role displayed as 'offline' until a NEW tool-call
      //      broadcast arrived over WS. That left pm + designer stuck
      //      at offline mid-session even while they were actively
      //      messaging, because chat messages don't update presence.
      //   The activity seed is small (50 events covers each role's
      //   latest state with wide margin) and activity events are
      //   single-word payloads, so the extra RTT is negligible.
      // Retry with backoff so the renderer tolerates a brief cold-
      // start window where the bpmux backend isn't yet listening
      // (Electron + bpmux backend launch in parallel; backend bind
      // can lag the renderer's first fetch by a few hundred ms).
      // Mirrors fetchBoard's retry pattern. Only the FINAL failure
      // logs to console.error so transient warmup blips don't
      // pollute the dev/test signal.
      const fetchHistoryWithRetry = async (url: string): Promise<HistoryMessage[]> => {
        const delays = [0, 200, 500]
        let lastErr: unknown = null
        for (const delay of delays) {
          if (controller.signal.aborted) throw new Error('aborted')
          if (delay > 0) {
            await new Promise<void>((resolve) => setTimeout(resolve, delay))
            if (controller.signal.aborted) throw new Error('aborted')
          }
          try {
            const res = await fetch(url, { headers, signal: controller.signal })
            if (!res.ok) throw new Error(`history fetch: ${res.status}`)
            return (await res.json()) as HistoryMessage[]
          } catch (err) {
            if ((err as { name?: string })?.name === 'AbortError') throw err
            lastErr = err
          }
        }
        throw lastErr instanceof Error ? lastErr : new Error(String(lastErr))
      }
      Promise.all([
        fetchHistoryWithRetry(`${apiBase}/api/messages?chat_only=true&limit=200`),
        fetchHistoryWithRetry(`${apiBase}/api/messages?message_type=activity&limit=50`)
      ])
        .then(([chat, activity]) => {
          if (unmountedRef.current) return
          // Both responses are newest-first from the server; combine
          // and chronological-sort before merging so the resulting
          // state stays ts-ordered regardless of fetch arrival order.
          const combined = [...chat, ...activity].sort((a, b) =>
            a.ts.localeCompare(b.ts)
          )
          setMessages((live) => {
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
            for (const m of live) {
              const k = messageKey(m)
              if (seen.has(k)) continue
              seen.add(k)
              merged.push(m)
            }
            return merged
          })
        })
        .catch((err) => {
          if (err?.name === 'AbortError') return
          console.error('bpmux ws: history fetch failed', err)
          historyLoadedRef.current = false
        })
    }

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
        if (socketRef.current !== sock) return
        backoffRef.current = INITIAL_BACKOFF_MS
        setStatus('connected')
        sendJson({
          type: 'identify',
          name: RENDERER_IDENTITY,
          tool: RENDERER_TOOL
        })
      }

      sock.onmessage = (ev) => {
        if (socketRef.current !== sock) return
        if (typeof ev.data === 'string') {
          handleServerMessage(ev.data)
        }
      }

      sock.onerror = (ev) => {
        if (socketRef.current !== sock) return
        console.error('bpmux ws: socket error', ev)
      }

      sock.onclose = (ev) => {
        if (socketRef.current !== sock) return
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
  }, [url, sendJson, handleServerMessage])

  return { status, messages, sessionId, sendChat }
}
