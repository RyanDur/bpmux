import { _electron as electron, type ElectronApplication, type Page } from '@playwright/test'
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import * as net from 'net'

export const getFreePort = async (): Promise<number> =>
  new Promise((resolve, reject) => {
    const server = net.createServer()
    server.on('error', reject)
    server.listen(0, () => {
      const addr = server.address()
      if (typeof addr === 'object' && addr !== null) {
        const port = addr.port
        server.close(() => resolve(port))
      } else {
        reject(new Error('failed to bind ephemeral port'))
      }
    })
  })

export interface LaunchOpts {
  userDataDir?: string
  sessionBase?: string
  backendPort?: number
  seedMessages?: ReadonlyArray<Record<string, unknown>>
  extraEnv?: Record<string, string>
}

export interface LaunchResult {
  app: ElectronApplication
  sessionBase: string
  userDataDir: string
  backendPort: number
}

export const launchIsolatedElectron = async (opts: LaunchOpts = {}): Promise<LaunchResult> => {
  const sessionBase = opts.sessionBase ?? mkdtempSync(join(tmpdir(), 'bpmux-playwright-'))
  const userDataDir =
    opts.userDataDir ?? mkdtempSync(join(tmpdir(), 'bpmux-playwright-userdata-'))
  const backendPort = opts.backendPort ?? (await getFreePort())
  if (opts.seedMessages && opts.seedMessages.length > 0) {
    const sessionId = 'playwright-seed-' + Date.now().toString(36)
    const sessionDir = join(sessionBase, sessionId)
    mkdirSync(sessionDir, { recursive: true })
    const jsonl = opts.seedMessages.map((m) => JSON.stringify(m)).join('\n') + '\n'
    writeFileSync(join(sessionDir, 'messages-current.jsonl'), jsonl)
    symlinkSync(sessionDir, join(sessionBase, 'current'))
  }
  const app = await electron.launch({
    args: ['out/main/index.js', `--user-data-dir=${userDataDir}`],
    env: {
      ...process.env,
      BPMUX_SESSION_BASE: sessionBase,
      BPMUX_BACKEND_PORT: String(backendPort),
      ...(opts.extraEnv ?? {})
    }
  })
  return { app, sessionBase, userDataDir, backendPort }
}

export const launchIsolatedWithWindow = async (
  opts: LaunchOpts = {}
): Promise<LaunchResult & { win: Page }> => {
  const result = await launchIsolatedElectron(opts)
  const win = await result.app.firstWindow()
  return { ...result, win }
}
