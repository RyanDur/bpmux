import type { Middleware } from './redux'
import type { StateSnapshot } from './reducer'
import { StateActionType, disconnected, type StateAction, type ConnectionConfig } from './actions'
import { dispatchServerFrame } from './decode'
import { encodeKeystroke } from './ptyCodec'
import { RENDERER_IDENTITY, RENDERER_TOOL } from './constants'
import { socketLifecycle } from './socketLifecycle'
import type { ClientMessage } from '@transport/types'

const sameConfig = (a: ConnectionConfig | null, b: ConnectionConfig): boolean =>
  a !== null &&
  a.url === b.url &&
  a.apiBase === b.apiBase &&
  a.authToken === b.authToken &&
  a.ownerNonce === b.ownerNonce

export const socketMiddleware: Middleware<StateSnapshot, StateAction> = (api) => {
  let messageSeq = 0
  let config: ConnectionConfig | null = null

  const lifecycle = socketLifecycle({
    dispatch: (action) => api.dispatch(action),
    onFrame: (raw) => dispatchServerFrame(raw, api.dispatch, () => (messageSeq += 1)),
    identify: () => ({
      type: 'identify',
      name: RENDERER_IDENTITY,
      tool: RENDERER_TOOL,
      ...(config?.ownerNonce ? { owner_nonce: config.ownerNonce } : {})
    })
  })

  const send = (msg: ClientMessage): void => lifecycle.send(msg)

  return (next) => (action) => {
    switch (action.type) {
      case StateActionType.StartRequested: {
        if (sameConfig(config, action.config)) return
        if (config !== null) lifecycle.teardown()
        config = action.config
        next(action)
        if (!action.config.url) {
          console.error('bpmux ws: no url provided via window.bpmux.wsUrl')
          api.dispatch(disconnected())
          return
        }
        lifecycle.connect(action.config.url)
        return
      }
      case StateActionType.ChatSendRequested: {
        const trimmed = action.text.trim()
        if (!trimmed) return
        const frame: ClientMessage = { type: 'send', text: trimmed }
        if (action.opts?.addressee) frame.addressee = action.opts.addressee
        if (action.opts?.to && action.opts.to.length > 0) frame.to = action.opts.to
        send(frame)
        return
      }
      case StateActionType.ReactionToggleRequested:
        send({
          type: 'reaction_toggle',
          channel: action.channel,
          message_key: action.messageKey,
          emoji: action.emoji,
          user: action.user
        })
        return
      case StateActionType.CommentsHydrateRequested:
        send({ type: 'comments_hydrate', story_id: action.storyId })
        return
      case StateActionType.TimelineHydrateRequested:
        send({ type: 'timeline_hydrate', story_id: action.storyId })
        return
      case StateActionType.HistoryHydrateRequested:
        send({
          type: 'history_hydrate',
          chat_limit: action.chatLimit,
          activity_limit: action.activityLimit
        })
        return
      case StateActionType.OlderHistoryRequested:
        send({
          type: 'history_page',
          request_id: action.requestId,
          before: action.before,
          limit: action.limit
        })
        return
      case StateActionType.OutcomeHorizonChangeRequested:
        send({
          type: 'set_outcome_horizon',
          outcome_id: action.outcomeId,
          horizon: action.horizon
        })
        return
      case StateActionType.OutcomeIterationReorderRequested:
        send({
          type: 'reorder_outcome_iteration',
          outcome_id: action.outcomeId,
          iteration_id: action.iterationId,
          position: action.position
        })
        return
      case StateActionType.OutcomeReprioritizeRequested:
        send({
          type: 'reprioritize_outcome',
          outcome_id: action.outcomeId,
          position: action.position
        })
        return
      case StateActionType.IterationReorderRequested:
        send({
          type: 'reorder_iteration',
          iteration_id: action.iterationId,
          position: action.position
        })
        return
      case StateActionType.AddStoryToIterationRequested:
        send({
          type: 'add_story_to_iteration',
          iteration_id: action.iterationId,
          story_id: action.storyId,
          position: action.position
        })
        return
      case StateActionType.PtySubscribeRequested:
        send({ type: 'pty_subscribe', terminal_id: action.terminalId })
        return
      case StateActionType.PtyInputRequested:
        send({
          type: 'pty_input',
          terminal_id: action.terminalId,
          data: encodeKeystroke(action.keystroke)
        })
        return
      case StateActionType.PtyResizeRequested:
        send({
          type: 'pty_resize',
          terminal_id: action.terminalId,
          cols: action.cols,
          rows: action.rows
        })
        return
      case StateActionType.FrameReceived:
        dispatchServerFrame(action.raw, api.dispatch, () => (messageSeq += 1))
        return
      case StateActionType.Reset:
        lifecycle.teardown()
        messageSeq = 0
        config = null
        next(action)
        return
      default:
        next(action)
    }
  }
}
