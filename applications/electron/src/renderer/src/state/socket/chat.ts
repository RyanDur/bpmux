import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import { StateActionType, type StateAction } from '../actions'
import type { ClientMessage } from '@transport/types'
import type { SocketDoor } from './door'
import { translating } from './translating'

export const chatSend =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.ChatSendRequested) return next(action)
    const trimmed = action.text.trim()
    if (!trimmed) return
    const frame: ClientMessage = { type: 'send', text: trimmed }
    if (action.opts?.addressee) frame.addressee = action.opts.addressee
    if (action.opts?.to && action.opts.to.length > 0) frame.to = action.opts.to
    door.send(frame)
  }

export const reactionToggle = translating((action) =>
  action.type === StateActionType.ReactionToggleRequested
    ? {
        type: 'reaction_toggle',
        channel: action.channel,
        message_key: action.messageKey,
        emoji: action.emoji,
        user: action.user
      }
    : null
)

export const olderHistory = translating((action) =>
  action.type === StateActionType.OlderHistoryRequested
    ? {
        type: 'history_page',
        request_id: action.requestId,
        before: action.before,
        limit: action.limit
      }
    : null
)
