import type { Middleware } from '../../lib/redux'
import type { SessionSnapshot } from './reducer'
import { SessionActionType, historyHydrateRequested, type SessionAction } from './actions'
import { HISTORY_ACTIVITY_LIMIT, HISTORY_CHAT_LIMIT } from './constants'

export const historyMiddleware: Middleware<SessionSnapshot, SessionAction> = (api) => {
  return (next) => (action) => {
    next(action)
    if (action.type === SessionActionType.SessionIdentified) {
      api.dispatch(historyHydrateRequested(HISTORY_CHAT_LIMIT, HISTORY_ACTIVITY_LIMIT))
    }
  }
}
