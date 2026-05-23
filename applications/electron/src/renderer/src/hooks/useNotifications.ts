import { useEffect, useRef } from 'react'
import { tryCatch } from '../lib/result'
import type { ChatMessage } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

export const isRelevant = (msg: ChatMessage, self: string): boolean => {
  if (shortName(msg.from) === self) return false
  if (msg.messageType === 'activity' || msg.messageType === 'system') return false
  if (msg.addressee === self) return true
  if (msg.to && msg.to.length > 0 && !msg.to.includes('all') && msg.to.includes(self)) return true
  return false
}

const isViewerActive = (): boolean => {
  if (typeof document === 'undefined') return true
  return document.hasFocus() && document.visibilityState === 'visible'
}

export const useNotifications = (messages: ChatMessage[]): void => {
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
    if (isViewerActive()) return
    if (typeof Notification === 'undefined' || Notification.permission !== 'granted') return

    const fresh = messages.slice(from)
    for (const msg of fresh) {
      if (!isRelevant(msg, RENDERER_IDENTITY)) continue
      const senderShort = shortName(msg.from)
      const titlePrefix =
        msg.addressee === RENDERER_IDENTITY
          ? `@mention from ${senderShort}`
          : `DM from ${senderShort}`
      tryCatch(
        () =>
          new Notification(titlePrefix, {
            body: msg.text.length > 140 ? `${msg.text.slice(0, 140)}…` : msg.text,
            tag: 'bpmux',
            silent: false
          }),
        (err) => err
      )
        .onSuccess((notification) => {
          notification.onclick = () => {
            window.focus()
            notification.close()
          }
        })
        .onFailure((err) => {
          console.error('bpmux notification: failed to construct', err)
        })
    }
  }, [messages])
}
