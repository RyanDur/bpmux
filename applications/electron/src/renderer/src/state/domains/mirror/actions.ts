import type { StateDeltaPayload } from '@transport/types'

export enum MirrorActionType {
  MirrorDeltaReceived = 'state/mirrorDeltaReceived'
}

export type MirrorAction = {
  type: MirrorActionType.MirrorDeltaReceived
  delta: StateDeltaPayload
  ts: string
}

export const mirrorDeltaReceived = (delta: StateDeltaPayload, ts: string): MirrorAction => ({
  type: MirrorActionType.MirrorDeltaReceived,
  delta,
  ts
})
