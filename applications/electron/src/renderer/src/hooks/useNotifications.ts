import { useEffect, useRef } from 'react'
import type { ChatMessage } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

export function isRelevant(msg: ChatMessage, self: string): boolean {
  if (shortName(msg.from) === self) return false
  if (msg.messageType === 'activity' || msg.messageType === 'system') return false
  if (msg.addressee === self) return true
  if (msg.to && msg.to.length > 0 && !msg.to.includes('all') && msg.to.includes(self)) return true
  return false
}

// #382: live-query focus at fire time instead of tracking via focus/
// blur event-ref. The earlier event-tracked `focusedRef` got stuck
// false in Electron under conditions where blur fires but focus
// never follows (modal window, devtools toggle, some macOS mission-
// control sequences). Ryan observed notifications firing while the
// Electron window was visibly focused — focusedRef was lying.
// Direct query uses the authoritative browser state at the moment
// of decision; no event-subscription, no staleness risk.
// Visibility check layered on top so tab-hidden Electron (possible
// via multi-window setups or OS window-group hide) also qualifies
// as "not actively viewing."
function isViewerActive(): boolean {
  if (typeof document === 'undefined') return true
  return document.hasFocus() && document.visibilityState === 'visible'
}

export function useNotifications(messages: ChatMessage[]): void {
  const lastSeenRef = useRef<number>(messages.length)
  const firstRunRef = useRef<boolean>(true)

  useEffect(() => {
    if (typeof Notification === 'undefined') return
    if (Notification.permission === 'default') {
      void Notification.requestPermission()
    }
  }, [])

  useEffect(() => {
    if (firstRunRef.current) {
      firstRunRef.current = false
      lastSeenRef.current = messages.length
      return
    }
    const from = lastSeenRef.current
    lastSeenRef.current = messages.length
    if (messages.length <= from) return
    // Focus + visibility gate (#382 fix): query live state. Event-
    // tracked ref was getting stuck false; direct query is the
    // authoritative signal.
    if (isViewerActive()) return
    if (typeof Notification === 'undefined' || Notification.permission !== 'granted') return

    const fresh = messages.slice(from)
    for (const msg of fresh) {
      if (!isRelevant(msg, RENDERER_IDENTITY)) continue
      const senderShort = shortName(msg.from)
      const titlePrefix = msg.addressee === RENDERER_IDENTITY ? `@mention from ${senderShort}` : `DM from ${senderShort}`
      try {
        const notification = new Notification(titlePrefix, {
          body: msg.text.length > 140 ? `${msg.text.slice(0, 140)}…` : msg.text,
          tag: 'bpmux',
          silent: false
        })
        notification.onclick = () => {
          window.focus()
          notification.close()
        }
      } catch (err) {
        console.error('bpmux notification: failed to construct', err)
      }
    }
  }, [messages])
}
