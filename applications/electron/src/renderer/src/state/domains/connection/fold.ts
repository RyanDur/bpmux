import { caseOf } from '@ryandur/sand'
import { StateActionType, type StateAction } from '../../actions'
import type { ConnectionStatus } from '@transport/types'

export type ConnectionState = {
  status: ConnectionStatus
  reconnectTick: number
}

export const initialConnection: ConnectionState = {
  status: 'disconnected',
  reconnectTick: 0
}

const onActionType = caseOf('type')

export const connectionFold = (
  state: ConnectionState = initialConnection,
  action: StateAction
): ConnectionState =>
  onActionType<StateAction, ConnectionState>(action, {
    [StateActionType.Connecting]: () =>
      state.status === 'connecting' ? state : { ...state, status: 'connecting' },
    [StateActionType.Connected]: () => ({ ...state, status: 'connected' }),
    [StateActionType.Reconnected]: () => ({
      status: 'connected',
      reconnectTick: state.reconnectTick + 1
    }),
    [StateActionType.Disconnected]: () => ({ ...state, status: 'disconnected' })
  }).orElse(state)
