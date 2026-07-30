export enum SocketActionType {
  StartRequested = 'state/startRequested',
  FrameReceived = 'state/frameReceived',
  Reset = 'state/reset'
}

export type ConnectionConfig = {
  url: string | undefined
  apiBase?: string
  authToken?: string
  ownerNonce?: string
}

export type SocketAction =
  | { type: SocketActionType.StartRequested; config: ConnectionConfig }
  | { type: SocketActionType.FrameReceived; raw: string }
  | { type: SocketActionType.Reset }

export const startRequested = (config: ConnectionConfig): SocketAction => ({
  type: SocketActionType.StartRequested,
  config
})

export const frameReceived = (raw: string): SocketAction => ({
  type: SocketActionType.FrameReceived,
  raw
})

export const reset = (): SocketAction => ({ type: SocketActionType.Reset })
