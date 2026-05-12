// #1303 Epic #1247 slice 1: per-test setup file. Polyfills WebSocket
// in jsdom env using `ws` so the renderer's transport/useWebSocket can
// actually connect to the real bpmux backend WS endpoint (jsdom's
// built-in WebSocket is a no-op stub that doesn't speak the protocol).
// Sister to globalSetup.ts which spawns the backend; this file wires
// up the per-jsdom-context globals tests need.
//
// BrowserCompatWebSocket subclass mirrors Chromium WebSocket semantic
// for close-during-CONNECTING state: production browser silently aborts
// the in-flight connect; the bare `ws` polyfill emits an `error` event
// ("WebSocket was closed before the connection was established"). Without
// the mirror, renderer cleanup hooks (e.g., useWebSocket's unmount path)
// surface noise on every test teardown — violating
// `feedback_no_console_errors_non_negotiable`. The subclass attaches a
// one-shot no-op error listener BEFORE calling super.close() so the
// abort-event is consumed quietly.

import { WebSocket as RealWebSocket } from 'ws'

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
