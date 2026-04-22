import { useEffect, useRef } from 'react'
import type { ChatMessage } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function isRelevant(msg: ChatMessage, self: string): boolean {
  if (shortName(msg.from) === self) return false
  if (msg.messageType === 'activity' || msg.messageType === 'system') return false
  if (msg.addressee === self) return true
  if (msg.to && msg.to.length > 0 && !msg.to.includes('all') && msg.to.includes(self)) return true
  return false
}

export function useNotifications(messages: ChatMessage[]): void {
  const lastSeenRef = useRef<number>(messages.length)
  const focusedRef = useRef<boolean>(typeof document !== 'undefined' ? document.hasFocus() : true)

  useEffect(() => {
    const onFocus = (): void => {
      focusedRef.current = true
    }
    const onBlur = (): void => {
      focusedRef.current = false
    }
    window.addEventListener('focus', onFocus)
    window.addEventListener('blur', onBlur)
    return () => {
      window.removeEventListener('focus', onFocus)
      window.removeEventListener('blur', onBlur)
    }
  }, [])

  useEffect(() => {
    if (typeof Notification === 'undefined') return
    if (Notification.permission === 'default') {
      void Notification.requestPermission()
    }
  }, [])

  useEffect(() => {
    const from = lastSeenRef.current
    lastSeenRef.current = messages.length
    if (messages.length <= from) return
    if (focusedRef.current) return
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
