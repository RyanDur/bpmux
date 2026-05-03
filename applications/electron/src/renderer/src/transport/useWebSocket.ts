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

// 500 is an empirical default: viewport rarely shows ~50 messages at
// 1080p / 12px font, 500 gives 10x headroom for light scroll-up before
// paged-fetch kicks in. Cap fires when length exceeds 2 * MAX_MESSAGES
// (1000) AND user is at-bottom — mid-scroll-up the array is left
// alone to preserve the user's reading position, per #1153 AC
// "Drop-tail when array exceeds 2L AND user is near-bottom".
export const MAX_MESSAGES = 500
export const CAP_TRIGGER_THRESHOLD = MAX_MESSAGES * 2

export interface SendOptions {
  addressee?: string | null
  to?: string[] | null
}

export interface UseWebSocketOptions {
  apiBase?: string
  authToken?: string
  // True when MessageList's scroll position is at-or-near the visual
  // bottom (column-reverse: |scrollTop| <= 8px). The cap suspends
  // while false so trimming doesn't disrupt a user reading older
  // messages. Default true (empty list = at bottom).
  atBottom?: boolean
  // Pin keys exempt from drop-tail eviction. Format `${ts}|${from}` —
  // matches Chat's pinKey. Pinned messages outside the most-recent
  // window are preserved when the cap fires.
  pinnedKeys?: ReadonlySet<string>
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

export function capMessages(messages: ChatMessage[], max: number): ChatMessage[] {
  return messages.length > max ? messages.slice(messages.length - max) : messages
}

const pinKey = (m: ChatMessage): string => `${m.ts}|${m.from}`

// Drop-tail cap that preserves pinned messages outside the most-recent
// window. Pin-rendering relies on the message being in the array, so
// trimming pinned messages would break pin glyphs and the pinned-list
// drawer. Pinned-but-old messages re-prepend in chronological order.
export function capMessagesPreservingPinned(
  messages: ChatMessage[],
  max: number,
  pinnedKeys: ReadonlySet<string>
): ChatMessage[] {
  if (messages.length <= max) return messages
  const recentN = messages.slice(messages.length - max)
  const recentKeys = new Set(recentN.map(pinKey))
  const olderPinned: ChatMessage[] = []
  for (const m of messages.slice(0, messages.length - max)) {
    const k = pinKey(m)
    if (pinnedKeys.has(k) && !recentKeys.has(k)) olderPinned.push(m)
  }
  return olderPinned.length === 0 ? recentN : [...olderPinned, ...recentN]
}

// Per-push cap with UX-guard: only fires when at-bottom (preserves
// scroll-position for users reading older history) AND length exceeds
// the trigger threshold (2L per AC). Pinned messages exempt.
export function capPerPush(
  next: ChatMessage[],
  atBottom: boolean,
  pinnedKeys: ReadonlySet<string>
): ChatMessage[] {
  if (!atBottom) return next
  if (next.length <= CAP_TRIGGER_THRESHOLD) return next
  return capMessagesPreservingPinned(next, MAX_MESSAGES, pinnedKeys)
}

export function useWebSocket(
  url: string | undefined,
  opts: UseWebSocketOptions = {}
): UseWebSocketResult {
  const { apiBase, authToken, atBottom = true, pinnedKeys } = opts
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

  // Mirror atBottom + pinnedKeys into refs so handleServerMessage's
  // closure sees the latest values without re-binding the WS handler
  // on every render. Reading state directly would freeze the values
  // at the time the callback was created.
  const atBottomRef = useRef<boolean>(atBottom)
  const pinnedKeysRef = useRef<ReadonlySet<string>>(pinnedKeys ?? new Set())
  useEffect(() => {
    atBottomRef.current = atBottom
  }, [atBottom])
  useEffect(() => {
    pinnedKeysRef.current = pinnedKeys ?? new Set()
  }, [pinnedKeys])

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
          return capPerPush([...prev, chat], atBottomRef.current, pinnedKeysRef.current)
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
            // #1153: same MAX_MESSAGES cap as the per-push path so
            // initial-load + history-fetch can't overshoot the bound.
            return capMessages(merged, MAX_MESSAGES)
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
