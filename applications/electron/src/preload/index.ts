import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron'

type BpmuxConfig = {
  wsUrl: string
  authToken: string
  ownerNonce: string
  apiBase: string
  e2e: boolean
}

type QuoteCommentPayload = {
  commentId: string
  author: string
  text: string
}

const config = ipcRenderer.sendSync('bpmux:config-sync') as BpmuxConfig

try {
  contextBridge.exposeInMainWorld('bpmux', {
    wsUrl: config.wsUrl,
    authToken: config.authToken,
    ownerNonce: config.ownerNonce,
    apiBase: config.apiBase,
    e2e: config.e2e,
    onQuoteComment: (callback: (payload: QuoteCommentPayload) => void): (() => void) => {
      const listener = (_event: IpcRendererEvent, payload: QuoteCommentPayload): void => {
        callback(payload)
      }
      ipcRenderer.on('bpmux:quote-comment', listener)
      return () => {
        ipcRenderer.removeListener('bpmux:quote-comment', listener)
      }
    }
  })
} catch (error) {
  console.error(error)
}
