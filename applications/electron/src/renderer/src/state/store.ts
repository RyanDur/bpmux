import { createStore, type Store } from './redux'
import { emptySnapshot, stateReducer, type StateSnapshot } from './reducer'
import { reset, type StateAction } from './actions'
import { socketListeners } from './socket'
import { emitListener } from './emitListener'
import { resetStateEmitter } from './emitter'
import { stopSentFrameCapture } from './testCapture'

const makeStore = (): Store<StateSnapshot, StateAction> => {
  const store = createStore(stateReducer, emptySnapshot(undefined))
  for (const listener of socketListeners()) store.listen(listener)
  store.listen(emitListener)
  return store
}

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
