import { StateActionType } from '../actions'
import type { ClientMessage } from '@transport/types'
import type { Translator } from './translating'

export const chatSend: Translator = (action) => {
  if (action.type !== StateActionType.ChatSendRequested) return null
  const trimmed = action.text.trim()
  if (!trimmed) return null
  const frame: ClientMessage = { type: 'send', text: trimmed }
  if (action.opts?.addressee) frame.addressee = action.opts.addressee
  if (action.opts?.to && action.opts.to.length > 0) frame.to = action.opts.to
  return frame
}

export const reactionToggle: Translator = (action) =>
  action.type === StateActionType.ReactionToggleRequested
    ? {
        type: 'reaction_toggle',
        channel: action.channel,
        message_key: action.messageKey,
        emoji: action.emoji,
        user: action.user
      }
    : null

export const olderHistory: Translator = (action) =>
  action.type === StateActionType.OlderHistoryRequested
    ? {
        type: 'history_page',
        request_id: action.requestId,
        before: action.before,
        limit: action.limit
      }
    : null
