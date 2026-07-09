import type { ChatMessage } from '../types'

const liveMessageListeners = new Set<(m: ChatMessage) => void>()
const ptyOutputListeners = new Map<string, Set<(data: string) => void>>()
const ptyModeListeners = new Map<string, Set<(bracketedPaste: boolean) => void>>()
const ptyExitListeners = new Set<(terminalId: string) => void>()

export const emitLiveMessage = (msg: ChatMessage): void => {
  for (const handler of liveMessageListeners) handler(msg)
}

export const emitPtyOutput = (terminalId: string, data: string): void => {
  const handlers = ptyOutputListeners.get(terminalId)
  if (!handlers) return
  for (const handler of handlers) handler(data)
}

export const emitPtyMode = (terminalId: string, bracketedPaste: boolean): void => {
  const handlers = ptyModeListeners.get(terminalId)
  if (!handlers) return
  for (const handler of handlers) handler(bracketedPaste)
}

export const emitPtyExit = (terminalId: string): void => {
  for (const handler of ptyExitListeners) handler(terminalId)
}

export const subscribeLiveMessages = (handler: (m: ChatMessage) => void): (() => void) => {
  liveMessageListeners.add(handler)
  return () => {
    liveMessageListeners.delete(handler)
  }
}

export const subscribePtyOutput = (
  terminalId: string,
  handler: (data: string) => void
): (() => void) => {
  let handlers = ptyOutputListeners.get(terminalId)
  if (!handlers) {
    handlers = new Set()
    ptyOutputListeners.set(terminalId, handlers)
  }
  handlers.add(handler)
  return () => {
    handlers.delete(handler)
  }
}

export const subscribePtyMode = (
  terminalId: string,
  handler: (bracketedPaste: boolean) => void
): (() => void) => {
  let handlers = ptyModeListeners.get(terminalId)
  if (!handlers) {
    handlers = new Set()
    ptyModeListeners.set(terminalId, handlers)
  }
  handlers.add(handler)
  return () => {
    handlers.delete(handler)
  }
}

export const subscribePtyExit = (handler: (terminalId: string) => void): (() => void) => {
  ptyExitListeners.add(handler)
  return () => {
    ptyExitListeners.delete(handler)
  }
}

export const resetStateEmitter = (): void => {
  liveMessageListeners.clear()
  ptyOutputListeners.clear()
  ptyModeListeners.clear()
  ptyExitListeners.clear()
}
