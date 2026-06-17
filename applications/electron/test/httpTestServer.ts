import { createServer, type IncomingMessage, type ServerResponse } from 'node:http'
import type { AddressInfo } from 'node:net'

export type TestHttpHandler = (req: IncomingMessage, res: ServerResponse) => void

export interface TestHttpServer {
  baseUrl: string
  close: () => Promise<void>
}

export const startTestHttpServer = async (handler: TestHttpHandler): Promise<TestHttpServer> => {
  const server = createServer(handler)
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  const { port } = server.address() as AddressInfo
  return {
    baseUrl: `http://127.0.0.1:${port}`,
    close: () =>
      new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())))
  }
}

export const respondJson = (res: ServerResponse, status: number, body: unknown): void => {
  res.statusCode = status
  res.setHeader('Content-Type', 'application/json')
  res.end(JSON.stringify(body))
}
