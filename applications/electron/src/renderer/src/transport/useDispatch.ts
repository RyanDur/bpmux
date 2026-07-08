import { dispatchState } from './state/store'
import type { StateAction } from './state/actions'

export const useDispatch = (): ((action: StateAction) => void) => dispatchState
