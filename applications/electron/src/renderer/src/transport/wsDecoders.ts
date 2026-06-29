import * as D from 'schemawax'
import {
  type ServerChatMessage,
  type ServerCommentsHydrated,
  type ServerError,
  type ServerHistoryHydrated,
  type ServerHistoryPageLoaded,
  type ServerPong,
  type ServerPtyExit,
  type ServerPtyMode,
  type ServerPtyOutput,
  type ServerTimelineHydrated,
  type ServerWelcome
} from './types'
import { stateDeltaDecoder, stateSnapshotDecoder } from './stateSyncDecoders'
import { asStoryId } from '../board/types'

const storyId = D.number.andThen(asStoryId)

export const welcomeDecoder: D.Decoder<ServerWelcome> = D.object({
  required: { type: D.literal('welcome'), session_id: D.string }
})

export const errorDecoder: D.Decoder<ServerError> = D.object({
  required: { type: D.literal('error'), reason: D.string }
})

export const pongDecoder: D.Decoder<ServerPong> = D.object({
  required: { type: D.literal('pong') }
})

export const ptyOutputDecoder: D.Decoder<ServerPtyOutput> = D.object({
  required: { type: D.literal('pty_output'), terminal_id: D.string, data: D.string }
})

export const ptyExitDecoder: D.Decoder<ServerPtyExit> = D.object({
  required: { type: D.literal('pty_exit'), terminal_id: D.string }
})

export const ptyModeDecoder: D.Decoder<ServerPtyMode> = D.object({
  required: { type: D.literal('pty_mode'), terminal_id: D.string, bracketed_paste: D.boolean }
})

export const commentsHydratedDecoder: D.Decoder<ServerCommentsHydrated> = D.object({
  required: {
    type: D.literal('comments_hydrated'),
    story_id: storyId,
    comments: D.array(
      D.object({
        required: {
          id: D.number,
          author: D.string,
          body: D.string,
          created_at: D.string
        }
      })
    )
  }
})

export const timelineHydratedDecoder: D.Decoder<ServerTimelineHydrated> = D.object({
  required: {
    type: D.literal('timeline_hydrated'),
    story_id: storyId,
    events: D.array(
      D.object({
        required: {
          event: D.string,
          actor: D.string,
          ts: D.string
        },
        optional: {
          label: D.string,
          body: D.string
        }
      })
    )
  }
})

export const historyHydratedDecoder: D.Decoder<ServerHistoryHydrated> = D.object({
  required: {
    type: D.literal('history_hydrated'),
    messages: D.array(
      D.object({
        required: { from: D.string, text: D.string, ts: D.string },
        optional: {
          to: D.nullable(D.array(D.string)),
          addressee: D.nullable(D.string),
          message_type: D.nullable(D.string)
        }
      })
    )
  }
})

export const historyPageLoadedDecoder: D.Decoder<ServerHistoryPageLoaded> = D.object({
  required: {
    type: D.literal('history_page_loaded'),
    request_id: D.number,
    messages: D.array(
      D.object({
        required: { from: D.string, text: D.string, ts: D.string },
        optional: {
          to: D.nullable(D.array(D.string)),
          addressee: D.nullable(D.string),
          message_type: D.nullable(D.string)
        }
      })
    )
  }
})

export const messageDecoder: D.Decoder<ServerChatMessage> = D.object({
  required: { type: D.literal('message'), from: D.string, text: D.string, ts: D.string },
  optional: {
    to: D.nullable(D.array(D.string)),
    addressee: D.nullable(D.string),
    message_type: D.nullable(D.string),
    state_delta: D.oneOf(stateDeltaDecoder, D.succeed(null)),
    state_snapshot: D.nullable(stateSnapshotDecoder)
  }
})
