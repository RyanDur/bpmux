export enum ConnectionActionType {
  Connecting = 'state/connecting',
  Connected = 'state/connected',
  Reconnected = 'state/reconnected',
  Disconnected = 'state/disconnected'
}

export type ConnectionAction =
  | { type: ConnectionActionType.Connecting }
  | { type: ConnectionActionType.Connected }
  | { type: ConnectionActionType.Reconnected }
  | { type: ConnectionActionType.Disconnected }

export const connecting = (): ConnectionAction => ({ type: ConnectionActionType.Connecting })
export const connected = (): ConnectionAction => ({ type: ConnectionActionType.Connected })
export const reconnected = (): ConnectionAction => ({ type: ConnectionActionType.Reconnected })
export const disconnected = (): ConnectionAction => ({ type: ConnectionActionType.Disconnected })
