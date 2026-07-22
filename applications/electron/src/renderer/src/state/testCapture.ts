import type { ClientMessage } from '../transport/types'

let captured: ClientMessage[] | null = null

export const captureSentFrame = (msg: ClientMessage): void => {
  if (captured) captured.push(msg)
}

export const startSentFrameCapture = (): ClientMessage[] => {
  captured = []
  return captured
}

export const stopSentFrameCapture = (): void => {
  captured = null
}
