import { StateActionType, type StateAction } from '../../actions'
import type { ConnectionStatus } from '@transport/types'

export type ConnectionSlice = {
  status: ConnectionStatus
  reconnectTick: number
}

export const emptyConnection = (url: string | undefined): ConnectionSlice => ({
  status: url ? 'connecting' : 'disconnected',
  reconnectTick: 0
})

export const connectionFold = (
  slice: ConnectionSlice,
  action: StateAction
): ConnectionSlice | null => {
  switch (action.type) {
    case StateActionType.Connecting:
      return slice.status === 'connecting' ? null : { ...slice, status: 'connecting' }
    case StateActionType.Connected:
      return { ...slice, status: 'connected' }
    case StateActionType.Reconnected:
      return { status: 'connected', reconnectTick: slice.reconnectTick + 1 }
    case StateActionType.Disconnected:
      return { ...slice, status: 'disconnected' }
    default:
      return null
  }
}
