import * as D from 'schemawax'
import {
  type ServerChatMessage,
  type ServerError,
  type ServerPong,
  type ServerPtyExit,
  type ServerPtyOutput,
  type ServerWelcome
} from './types'
import { stateDeltaDecoder, stateSnapshotDecoder } from './stateSyncDecoders'

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
