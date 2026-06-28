import { dispatchSession } from './session/store'
import type { SessionAction } from './session/actions'

export const useDispatch = (): ((action: SessionAction) => void) => dispatchSession
