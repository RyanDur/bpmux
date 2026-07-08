import { useSyncExternalStore } from 'react'
import { getStateSnapshot, subscribeState } from './state/store'
import type { StateSnapshot } from './state/reducer'

const selectionCache = new WeakMap<StateSnapshot, Map<(state: StateSnapshot) => unknown, unknown>>()

const select = <T>(state: StateSnapshot, selector: (state: StateSnapshot) => T): T => {
  let bySelector = selectionCache.get(state)
  if (!bySelector) {
    bySelector = new Map()
    selectionCache.set(state, bySelector)
  }
  if (!bySelector.has(selector)) {
    bySelector.set(selector, selector(state))
  }
  return bySelector.get(selector) as T
}

export const useSelector = <T>(selector: (state: StateSnapshot) => T): T =>
  useSyncExternalStore(
    subscribeState,
    () => select(getStateSnapshot(), selector),
    () => select(getStateSnapshot(), selector)
  )
