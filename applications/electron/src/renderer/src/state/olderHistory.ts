import type { HistoryMessageWire } from '../transport/types'

type PendingRequest = {
  resolve: (messages: HistoryMessageWire[]) => void
  reject: (reason: Error) => void
}

let requestCounter = 0
const pending = new Map<number, PendingRequest>()

export const nextOlderHistoryRequest = (): {
  requestId: number
  promise: Promise<HistoryMessageWire[]>
} => {
  const requestId = (requestCounter += 1)
  const promise = new Promise<HistoryMessageWire[]>((resolve, reject) => {
    pending.set(requestId, { resolve, reject })
  })
  return { requestId, promise }
}

export const resolveOlderHistory = (requestId: number, messages: HistoryMessageWire[]): void => {
  const entry = pending.get(requestId)
  if (!entry) return
  pending.delete(requestId)
  entry.resolve(messages)
}

export const rejectPendingOlderHistory = (reason: string): void => {
  const error = new Error(reason)
  for (const entry of pending.values()) entry.reject(error)
  pending.clear()
}
