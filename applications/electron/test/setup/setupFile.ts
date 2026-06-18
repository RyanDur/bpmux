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

const mediaHost = globalThis as unknown as { matchMedia?: (query: string) => MediaQueryList }
mediaHost.matchMedia = (query: string): MediaQueryList =>
  ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false
  }) as unknown as MediaQueryList

const canvasCtor = (globalThis as unknown as { HTMLCanvasElement?: typeof HTMLCanvasElement })
  .HTMLCanvasElement
if (canvasCtor !== undefined) {
  const stub2dContext = {
    fillRect: () => {},
    clearRect: () => {},
    strokeRect: () => {},
    fillText: () => {},
    strokeText: () => {},
    measureText: () => ({ width: 8 }),
    getImageData: () => ({ data: new Uint8ClampedArray(4) }),
    putImageData: () => {},
    createImageData: () => ({ data: new Uint8ClampedArray(4) }),
    createLinearGradient: () => ({ addColorStop: () => {} }),
    setTransform: () => {},
    resetTransform: () => {},
    getTransform: () => ({}),
    drawImage: () => {},
    save: () => {},
    restore: () => {},
    beginPath: () => {},
    moveTo: () => {},
    lineTo: () => {},
    closePath: () => {},
    stroke: () => {},
    translate: () => {},
    scale: () => {},
    rotate: () => {},
    arc: () => {},
    fill: () => {},
    rect: () => {},
    clip: () => {}
  }
  canvasCtor.prototype.getContext = (() =>
    stub2dContext) as unknown as HTMLCanvasElement['getContext']
}
