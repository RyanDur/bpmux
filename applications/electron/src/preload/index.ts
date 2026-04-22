import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

interface BpmuxConfig {
  wsUrl: string
  authToken: string
  apiBase: string
}

// Preload runs after app.whenReady()+backend spawn, so the IPC handler is
// registered by the time this code executes. Fetch the config synchronously
// via sendSync so window.bpmux exposes real string fields rather than
// promises that the renderer would have to await.
const config = ipcRenderer.sendSync('bpmux:config-sync') as BpmuxConfig

const bpmux = {
  wsUrl: config.wsUrl,
  authToken: config.authToken,
  apiBase: config.apiBase
}

if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', electronAPI)
    contextBridge.exposeInMainWorld('api', {})
    contextBridge.exposeInMainWorld('bpmux', bpmux)
  } catch (error) {
    console.error(error)
  }
} else {
  // @ts-ignore (define in dts)
  window.electron = electronAPI
  // @ts-ignore (define in dts)
  window.api = {}
  // @ts-ignore (define in dts)
  window.bpmux = bpmux
}
