import { has } from '@ryandur/sand'
import type { Dispatch } from '../redux'
import { type ConnectionConfig, type StateAction, disconnected } from '../actions'
import { dispatchServerFrame } from '../decode'
import { RENDERER_IDENTITY, RENDERER_TOOL } from '../constants'
import { captureSentFrame } from '../testCapture'
import { type SocketLifecycle, socketLifecycle } from './lifecycle'
import type { ClientMessage } from '@transport/types'

export type SocketDoor = {
  alreadyOn(next: ConnectionConfig): boolean
  adopt(next: ConnectionConfig): void
  connect(dispatch: Dispatch<StateAction>): void
  send(msg: ClientMessage): void
  frame(raw: string, dispatch: Dispatch<StateAction>): void
  reset(): void
}

const sameConfig = (a: ConnectionConfig | null, b: ConnectionConfig): boolean =>
  a !== null &&
  a.url === b.url &&
  a.apiBase === b.apiBase &&
  a.authToken === b.authToken &&
  a.ownerNonce === b.ownerNonce

export const socketDoor = (): SocketDoor => {
  let messageSeq = 0
  let config: ConnectionConfig | null = null
  let lifecycle: SocketLifecycle | null = null

  const frame = (raw: string, dispatch: Dispatch<StateAction>): void =>
    dispatchServerFrame(raw, dispatch, () => (messageSeq += 1))

  return {
    alreadyOn: (next) => sameConfig(config, next),
    adopt: (next) => {
      if (config !== null) lifecycle?.teardown()
      config = next
    },
    connect: (dispatch) => {
      const url = config?.url
      if (!has(url)) {
        console.error('bpmux ws: no url provided via window.bpmux.wsUrl')
        dispatch(disconnected())
        return
      }
      lifecycle = socketLifecycle({
        dispatch,
        onFrame: (raw) => frame(raw, dispatch),
        identify: () => ({
          type: 'identify',
          name: RENDERER_IDENTITY,
          tool: RENDERER_TOOL,
          ...(config?.ownerNonce ? { owner_nonce: config.ownerNonce } : {})
        })
      })
      lifecycle.connect(url)
    },
    send: (msg) => {
      captureSentFrame(msg)
      if (!lifecycle) {
        console.warn('bpmux ws: send dropped, socket not started', msg)
        return
      }
      lifecycle.send(msg)
    },
    frame,
    reset: () => {
      lifecycle?.teardown()
      lifecycle = null
      messageSeq = 0
      config = null
    }
  }
}
