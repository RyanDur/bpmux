import { contextBridge, ipcRenderer } from 'electron'

interface BpmuxConfig {
  wsUrl: string
  authToken: string
  apiBase: string
}

const config = ipcRenderer.sendSync('bpmux:config-sync') as BpmuxConfig

try {
  contextBridge.exposeInMainWorld('bpmux', {
    wsUrl: config.wsUrl,
    authToken: config.authToken,
    apiBase: config.apiBase
  })
} catch (error) {
  console.error(error)
}
