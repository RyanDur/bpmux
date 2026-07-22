import { dispatchState } from './store'
import type { StateAction } from './actions'

export const useDispatch = (): ((action: StateAction) => void) => dispatchState
