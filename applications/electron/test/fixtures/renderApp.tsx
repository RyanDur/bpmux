import { render, type RenderResult } from '@testing-library/react'
import { inject } from 'vitest'

import App from '@renderer/App'

import { installBpmuxShim, uninstallBpmuxShim } from './bpmuxShim'

export interface RenderAppOptions {
  port?: number
  authToken?: string
  apiBase?: string
  wsUrl?: string
}

export interface RenderAppResult extends RenderResult {
  cleanup: () => void
  port: number
  authToken: string
  apiBase: string
}

export function renderApp(opts: RenderAppOptions = {}): RenderAppResult {
  const port = opts.port ?? inject('bpmuxBackendPort')
  const authToken = opts.authToken ?? inject('bpmuxAuthToken')
  const apiBase = opts.apiBase ?? `http://127.0.0.1:${port}`
  const wsUrl = opts.wsUrl ?? `ws://127.0.0.1:${port}/ws`

  installBpmuxShim({ wsUrl, authToken, apiBase })
  const result = render(<App />)
  return {
    ...result,
    port,
    authToken,
    apiBase,
    cleanup: () => {
      result.unmount()
      uninstallBpmuxShim()
    }
  }
}
