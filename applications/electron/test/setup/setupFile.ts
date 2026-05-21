import { configure } from '@testing-library/react'
import { WebSocket as RealWebSocket } from 'ws'

configure({ testIdAttribute: 'data-test', defaultHidden: true })

class BrowserCompatWebSocket extends RealWebSocket {
  close(code?: number, reason?: string): void {
    if (this.readyState === RealWebSocket.CONNECTING) {
      this.once('error', () => {})
    }
    super.close(code, reason)
  }
}

const w = globalThis as unknown as { WebSocket: typeof RealWebSocket }
w.WebSocket = BrowserCompatWebSocket as unknown as typeof RealWebSocket
