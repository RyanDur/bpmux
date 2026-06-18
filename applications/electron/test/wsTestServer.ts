import { WebSocket, WebSocketServer, type WebSocket as WsSocket } from 'ws'
import type { AddressInfo } from 'node:net'

const ignoreConnectAbort = (): void => undefined

export class BrowserWebSocket extends WebSocket {
  close(code?: number, reason?: string): void {
    if (this.readyState === WebSocket.CONNECTING) {
      this.on('error', ignoreConnectAbort)
      this.terminate()
      return
    }
    super.close(code, reason)
  }
}

export interface TestWsConnection {
  send: (data: string) => void
  close: (code?: number, reason?: string) => void
  drop: () => void
  received: string[]
}

export interface TestWsServer {
  url: string
  waitForConnection: () => Promise<TestWsConnection>
  connectionCount: () => number
  close: () => Promise<void>
}

export const startTestWsServer = async (): Promise<TestWsServer> => {
  const wss = new WebSocketServer({ port: 0, host: '127.0.0.1' })
  await new Promise<void>((resolve) => wss.once('listening', resolve))
  const { port } = wss.address() as AddressInfo

  const connections: TestWsConnection[] = []
  const waiters: ((conn: TestWsConnection) => void)[] = []
  let handedOut = 0

  const handOut = (): void => {
    while (waiters.length > 0 && handedOut < connections.length) {
      const resolve = waiters.shift() as (conn: TestWsConnection) => void
      resolve(connections[handedOut])
      handedOut += 1
    }
  }

  wss.on('connection', (socket: WsSocket) => {
    const received: string[] = []
    socket.on('message', (data) => received.push(data.toString()))
    connections.push({
      send: (payload) => socket.send(payload),
      close: (code, reason) => socket.close(code, reason),
      drop: () => socket.terminate(),
      received
    })
    handOut()
  })

  return {
    url: `ws://127.0.0.1:${port}`,
    waitForConnection: () =>
      new Promise<TestWsConnection>((resolve) => {
        waiters.push(resolve)
        handOut()
      }),
    connectionCount: () => connections.length,
    close: () =>
      new Promise<void>((resolve, reject) => wss.close((err) => (err ? reject(err) : resolve())))
  }
}
