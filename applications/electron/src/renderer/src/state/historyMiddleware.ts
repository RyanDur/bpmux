import type { Middleware } from '../lib/redux'
import type { StateSnapshot } from './reducer'
import { StateActionType, historyHydrateRequested, type StateAction } from './actions'
import { HISTORY_ACTIVITY_LIMIT, HISTORY_CHAT_LIMIT } from './constants'

export const historyMiddleware: Middleware<StateSnapshot, StateAction> = (api) => {
  return (next) => (action) => {
    next(action)
    if (action.type === StateActionType.Welcomed) {
      api.dispatch(historyHydrateRequested(HISTORY_CHAT_LIMIT, HISTORY_ACTIVITY_LIMIT))
    }
  }
}
