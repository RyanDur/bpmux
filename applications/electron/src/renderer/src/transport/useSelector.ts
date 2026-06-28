import { useSyncExternalStore } from 'react'
import { getSessionSnapshot, subscribeSession } from './session/store'
import type { SessionSnapshot } from './session/reducer'

const selectionCache = new WeakMap<
  SessionSnapshot,
  Map<(state: SessionSnapshot) => unknown, unknown>
>()

const select = <T>(state: SessionSnapshot, selector: (state: SessionSnapshot) => T): T => {
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

export const useSelector = <T>(selector: (state: SessionSnapshot) => T): T =>
  useSyncExternalStore(
    subscribeSession,
    () => select(getSessionSnapshot(), selector),
    () => select(getSessionSnapshot(), selector)
  )
