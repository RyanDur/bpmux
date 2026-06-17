import { applyMiddleware, createStore, type Store } from '../../lib/redux'
import { emptySnapshot, sessionReducer, type SessionSnapshot } from './reducer'
import { reset, type SessionAction } from './actions'
import { socketMiddleware } from './socketMiddleware'
import { historyMiddleware } from './historyMiddleware'
import { emitMiddleware } from './emitMiddleware'
import { resetSessionEmitter } from './emitter'
import { stopSentFrameCapture } from './testCapture'

const makeStore = (): Store<SessionSnapshot, SessionAction> =>
  createStore(
    sessionReducer,
    emptySnapshot(undefined),
    applyMiddleware(socketMiddleware, historyMiddleware, emitMiddleware)
  )

let store = makeStore()

export const dispatchSession = (action: SessionAction): void => store.dispatch(action)

export const getSessionSnapshot = (): SessionSnapshot => store.getState()

export const subscribeSession = (listener: () => void): (() => void) => store.subscribe(listener)

export const resetSessionForTest = (): void => {
  store.dispatch(reset())
  resetSessionEmitter()
  stopSentFrameCapture()
  store = makeStore()
}
