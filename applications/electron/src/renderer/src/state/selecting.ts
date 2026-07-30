import { useSyncExternalStore } from 'react'
import { getStateSnapshot, subscribeState } from './store'

export const useSelector = <T>(
  selector: (snapshot: ReturnType<typeof getStateSnapshot>) => T
): T => {
  const snapshot = (): T => selector(getStateSnapshot())
  return useSyncExternalStore(subscribeState, snapshot, snapshot)
}
