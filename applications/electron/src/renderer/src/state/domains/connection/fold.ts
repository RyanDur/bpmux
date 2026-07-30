import { matchOn } from '@ryandur/sand'
import { ConnectionActionType } from './actions'
import type { StateAction } from '../../actions'
import type { ConnectionStatus } from '@transport/types'

export type ConnectionState = {
  status: ConnectionStatus
  reconnectTick: number
}

export const initialConnection: ConnectionState = {
  status: 'disconnected',
  reconnectTick: 0
}

const connectionTypes = Object.values(ConnectionActionType)
const matchConnection = matchOn(connectionTypes)

const isConnectionType = (candidate: StateAction['type']): candidate is ConnectionActionType =>
  (connectionTypes as StateAction['type'][]).includes(candidate)

export const connectionFold = (
  state: ConnectionState = initialConnection,
  action: StateAction
): ConnectionState =>
  matchConnection<ConnectionState>(isConnectionType(action.type) ? action.type : null, {
    [ConnectionActionType.Connecting]: () =>
      state.status === 'connecting' ? state : { ...state, status: 'connecting' },
    [ConnectionActionType.Connected]: () => ({ ...state, status: 'connected' }),
    [ConnectionActionType.Reconnected]: () => ({
      status: 'connected',
      reconnectTick: state.reconnectTick + 1
    }),
    [ConnectionActionType.Disconnected]: () => ({ ...state, status: 'disconnected' })
  }).orElse(state)
