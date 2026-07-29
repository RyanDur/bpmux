import { applyMiddleware, createStore, type Store } from './redux'
import { emptySnapshot, stateReducer, type StateSnapshot } from './reducer'
import { reset, type StateAction } from './actions'
import { socketMiddleware } from './socketMiddleware'
import { emitMiddleware } from './emitMiddleware'
import { resetStateEmitter } from './emitter'
import { stopSentFrameCapture } from './testCapture'

const makeStore = (): Store<StateSnapshot, StateAction> =>
  createStore(
    stateReducer,
    emptySnapshot(undefined),
    applyMiddleware(socketMiddleware, emitMiddleware)
  )

let store = makeStore()

export const dispatchState = (action: StateAction): void => store.dispatch(action)

export const getStateSnapshot = (): StateSnapshot => store.getState()

export const subscribeState = (listener: () => void): (() => void) => store.subscribe(listener)

export const resetStateForTest = (): void => {
  store.dispatch(reset())
  resetStateEmitter()
  stopSentFrameCapture()
  store = makeStore()
}
