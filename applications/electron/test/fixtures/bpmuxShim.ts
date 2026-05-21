export interface BpmuxShimConfig {
  wsUrl: string
  authToken: string
  apiBase: string
}

let previousBpmux: Window['bpmux'] | undefined
let installed = false

export const installBpmuxShim = (config: BpmuxShimConfig): void => {
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

export const uninstallBpmuxShim = (): void => {
  if (!installed) return
  window.bpmux = previousBpmux
  previousBpmux = undefined
  installed = false
}
