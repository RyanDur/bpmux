import { describeHttpError, http } from '../../api/http'
import { asyncTryCatch } from '../../lib/asyncResult'
import type { Middleware } from '../../lib/redux'
import type { SessionSnapshot } from './reducer'
import {
  SessionActionType,
  historyLoaded,
  type HistoryMessage,
  type SessionAction,
  type SessionConfig
} from './actions'
import { HISTORY_ACTIVITY_LIMIT, HISTORY_CHAT_LIMIT } from './constants'
import { MESSAGE_TYPE } from '../messageType'

export const historyMiddleware: Middleware<SessionSnapshot, SessionAction> = (api) => {
  let abort: AbortController | null = null
  let config: SessionConfig | null = null

  const load = (apiBase: string, authToken: string): void => {
    abort?.abort()
    const controller = new AbortController()
    abort = controller
    const fetchWithRetry = async (endpoint: string): Promise<HistoryMessage[]> => {
      const delays = [0, 200, 500]
      let lastErr: unknown = null
      for (const delay of delays) {
        if (controller.signal.aborted) throw new Error('aborted')
        if (delay > 0) {
          await new Promise<void>((resolve) => setTimeout(resolve, delay))
          if (controller.signal.aborted) throw new Error('aborted')
        }
        const result = await http.get<HistoryMessage[]>(
          { apiBase, authToken },
          endpoint,
          controller.signal
        ).value
        if (result.kind === 'success') return result.value
        if (result.reason.kind === 'network' && result.reason.message.includes('abort')) {
          throw new Error('aborted')
        }
        lastErr = new Error(describeHttpError(result.reason))
      }
      throw lastErr instanceof Error ? lastErr : new Error(String(lastErr))
    }
    void asyncTryCatch(() =>
      Promise.all([
        fetchWithRetry(`/api/messages?chat_only=true&limit=${HISTORY_CHAT_LIMIT}`),
        fetchWithRetry(
          `/api/messages?message_type=${MESSAGE_TYPE.Activity}&limit=${HISTORY_ACTIVITY_LIMIT}`
        )
      ])
    )
      .onSuccess(([chat, activity]) => {
        if (controller.signal.aborted) return
        api.dispatch(historyLoaded([...chat, ...activity].sort((a, b) => a.ts.localeCompare(b.ts))))
      })
      .onFailure((err) => {
        if (err.name === 'AbortError' || err.message === 'aborted') return
        console.error('bpmux session: history fetch failed', err)
      }).value
  }

  return (next) => (action) => {
    next(action)
    if (action.type === SessionActionType.StartRequested) {
      config = action.config
      if (action.config.apiBase && action.config.authToken) {
        load(action.config.apiBase, action.config.authToken)
      }
    } else if (action.type === SessionActionType.Reconnected) {
      if (config?.apiBase && config?.authToken) load(config.apiBase, config.authToken)
    } else if (action.type === SessionActionType.Reset) {
      abort?.abort()
      abort = null
      config = null
    }
  }
}
