import { has, maybe } from '@ryandur/sand'
import { StateActionType } from '../actions'
import type { ClientMessage } from '@transport/types'
import type { Translator } from './translating'

export const chatSend: Translator = (action) => {
  if (action.type !== StateActionType.ChatSendRequested) return null
  return maybe(action.text.trim(), has)
    .map((text) => {
      const frame: ClientMessage = { type: 'send', text }
      if (has(action.opts?.addressee)) frame.addressee = action.opts.addressee
      if (has(action.opts?.to)) frame.to = action.opts.to
      return frame
    })
    .orNull()
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
