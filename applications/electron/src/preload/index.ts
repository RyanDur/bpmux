import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron'

interface BpmuxConfig {
  wsUrl: string
  authToken: string
  apiBase: string
}

interface QuoteCommentPayload {
  commentId: string
  author: string
  text: string
}

const config = ipcRenderer.sendSync('bpmux:config-sync') as BpmuxConfig

try {
  contextBridge.exposeInMainWorld('bpmux', {
    wsUrl: config.wsUrl,
    authToken: config.authToken,
    apiBase: config.apiBase,
    // #371: subscribe to main-process quote-comment dispatches
    // triggered by the native context-menu's "Quote in reply" item.
    // Returns an unsubscribe fn so hook cleanup is straightforward.
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
