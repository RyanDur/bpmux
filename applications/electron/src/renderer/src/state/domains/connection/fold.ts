import { caseOf } from '@ryandur/sand'
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

const onActionType = caseOf('type')

export const connectionFold = (
  slice: ConnectionSlice,
  action: StateAction
): ConnectionSlice | null =>
  onActionType<StateAction, ConnectionSlice | null>(action, {
    [StateActionType.Connecting]: () =>
      slice.status === 'connecting' ? null : { ...slice, status: 'connecting' },
    [StateActionType.Connected]: () => ({ ...slice, status: 'connected' }),
    [StateActionType.Reconnected]: () => ({
      status: 'connected',
      reconnectTick: slice.reconnectTick + 1
    }),
    [StateActionType.Disconnected]: () => ({ ...slice, status: 'disconnected' })
  }).orNull()
