// #1303 Epic #1247 slice 1: window.bpmux test-shim.
//
// In production, window.bpmux is injected by the preload script
// (preload/index.ts -> contextBridge.exposeInMainWorld) and provides
// { wsUrl, authToken, apiBase, onQuoteComment }. In RTL+real-backend
// tests there's no Electron preload (we run renderer-only in jsdom),
// so this shim installs an equivalent globalThis.window.bpmux pointing
// at the spawned backend's HTTP/WS endpoints. Renderer code reads
// window.bpmux unchanged.
//
// Lifecycle: tests call installBpmuxShim() before render, uninstall
// after. The fixture's renderApp() wires this automatically. Tests can
// call directly when they need to assert the absent-shim fallback path
// (e.g., Board.tsx's mock-stories branch).

export interface BpmuxShimConfig {
  wsUrl: string
  authToken: string
  apiBase: string
}

let previousBpmux: Window['bpmux'] | undefined
let installed = false

export function installBpmuxShim(config: BpmuxShimConfig): void {
  if (installed) {
    uninstallBpmuxShim()
  }
  previousBpmux = window.bpmux
  window.bpmux = {
    wsUrl: config.wsUrl,
    authToken: config.authToken,
    apiBase: config.apiBase,
    onQuoteComment: () => () => {}
  }
  installed = true
}

export function uninstallBpmuxShim(): void {
  if (!installed) return
  window.bpmux = previousBpmux
  previousBpmux = undefined
  installed = false
}
