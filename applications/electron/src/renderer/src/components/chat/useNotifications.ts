import { useEffect } from 'react'
import { tryCatch } from '@ryandur/sand'
import { MESSAGE_TYPE } from '@transport/messageType'
import type { ChatMessage } from '@transport/types'
import { RENDERER_IDENTITY, subscribeLiveMessages } from '@state'

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

export const isRelevant = (msg: ChatMessage, self: string): boolean => {
  if (shortName(msg.from) === self) return false
  if (msg.messageType === MESSAGE_TYPE.Activity || msg.messageType === MESSAGE_TYPE.System)
    return false
  if (msg.addressee === self) return true
  if (msg.to && msg.to.length > 0 && !msg.to.includes('all') && msg.to.includes(self)) return true
  return false
}

const isViewerActive = (): boolean => {
  if (typeof document === 'undefined') return true
  return document.hasFocus() && document.visibilityState === 'visible'
}

const notify = (msg: ChatMessage): void => {
  if (isViewerActive()) return
  if (typeof Notification === 'undefined' || Notification.permission !== 'granted') return
  if (!isRelevant(msg, RENDERER_IDENTITY)) return
  const senderShort = shortName(msg.from)
  const titlePrefix =
    msg.addressee === RENDERER_IDENTITY ? `@mention from ${senderShort}` : `DM from ${senderShort}`
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

export const useNotifications = (): void => {
  useEffect(() => {
    if (typeof Notification === 'undefined') return
    if (Notification.permission === 'default') {
      void Notification.requestPermission()
    }
  }, [])

  useEffect(() => subscribeLiveMessages(notify), [])
}
