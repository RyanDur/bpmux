import { maybe } from '../../lib/maybe'
import { tryCatch } from '../../lib/result'
import type { Dispatch } from '../../lib/redux'
import { MESSAGE_TYPE, asMessageType } from '../messageType'
import {
  commentsHydratedDecoder,
  errorDecoder,
  historyHydratedDecoder,
  historyPageLoadedDecoder,
  messageDecoder,
  pongDecoder,
  ptyExitDecoder,
  ptyModeDecoder,
  ptyOutputDecoder,
  timelineHydratedDecoder,
  welcomeDecoder
} from '../wsDecoders'
import { resolveOlderHistory } from './olderHistory'
import {
  commentsHydrated,
  historyLoaded,
  messageReceived,
  mirrorDeltaReceived,
  mirrorSnapshotReceived,
  ptyExitReceived,
  ptyModeReceived,
  ptyOutputReceived,
  timelineHydrated,
  welcomed,
  type StateAction
} from './actions'
import { type ChatMessage, type ServerChatMessage } from '../types'

const chatFrameToAction = (
  message: ServerChatMessage,
  nextSeq: () => number
): StateAction | null => {
  if (message.message_type === MESSAGE_TYPE.StateSnapshot) {
    return message.state_snapshot ? mirrorSnapshotReceived(message.state_snapshot) : null
  }
  if (message.message_type === MESSAGE_TYPE.StateDelta) {
    return message.state_delta ? mirrorDeltaReceived(message.state_delta, message.ts) : null
  }
  const chat: ChatMessage = {
    id: `${message.ts}-${nextSeq()}`,
    from: message.from,
    text: message.text,
    ts: message.ts,
    to: message.to ?? null,
    addressee: message.addressee ?? null,
    messageType: asMessageType(message.message_type)
  }
  return messageReceived(chat)
}

export const dispatchServerFrame = (
  raw: string,
  dispatch: Dispatch<StateAction>,
  nextSeq: () => number
): void => {
  const parseResult = tryCatch(
    () => JSON.parse(raw) as unknown,
    (err) => err
  )
  if (parseResult.kind === 'failure') {
    console.error('bpmux ws: non-JSON frame', parseResult.reason, raw)
    return
  }
  const frame = parseResult.value

  const handled = maybe(welcomeDecoder.decode(frame))
    .map(() => dispatch(welcomed()))
    .or(() =>
      maybe(errorDecoder.decode(frame)).map((error) => {
        console.error('bpmux ws: server error', error.reason)
      })
    )
    .or(() => maybe(pongDecoder.decode(frame)).map(() => undefined))
    .or(() =>
      maybe(ptyOutputDecoder.decode(frame)).map((out) =>
        dispatch(ptyOutputReceived(out.terminal_id, out.data))
      )
    )
    .or(() =>
      maybe(ptyExitDecoder.decode(frame)).map((exit) => dispatch(ptyExitReceived(exit.terminal_id)))
    )
    .or(() =>
      maybe(ptyModeDecoder.decode(frame)).map((mode) =>
        dispatch(ptyModeReceived(mode.terminal_id, mode.bracketed_paste))
      )
    )
    .or(() =>
      maybe(commentsHydratedDecoder.decode(frame)).map((hydrated) =>
        dispatch(commentsHydrated(hydrated.story_id, hydrated.comments))
      )
    )
    .or(() =>
      maybe(timelineHydratedDecoder.decode(frame)).map((hydrated) =>
        dispatch(timelineHydrated(hydrated.story_id, hydrated.events))
      )
    )
    .or(() =>
      maybe(historyHydratedDecoder.decode(frame)).map((hydrated) =>
        dispatch(historyLoaded(hydrated.messages))
      )
    )
    .or(() =>
      maybe(historyPageLoadedDecoder.decode(frame)).map((page) =>
        resolveOlderHistory(page.request_id, page.messages)
      )
    )
    .or(() =>
      maybe(messageDecoder.decode(frame)).map((message) => {
        const action = chatFrameToAction(message, nextSeq)
        if (action) dispatch(action)
      })
    )
  if (handled.isNothing) {
    console.error('bpmux ws: unknown frame', frame)
  }
}
