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
  /**
   * Pre-seed `<session-dir>/pins.json` so the Electron renderer's
   * `usePins` hook hydrates a non-empty pins state on mount (#489).
   * Map shape: channel key → ordered Pin records (matches
   * `session::pins::PinsStore` Rust shape on the wire).
   */
  seedPins?: Record<string, ReadonlyArray<Record<string, unknown>>>
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
  const wantsSeed =
    (opts.seedMessages && opts.seedMessages.length > 0) ||
    (opts.seedPins && Object.keys(opts.seedPins).length > 0)
  if (wantsSeed) {
    const sessionId = 'playwright-seed-' + Date.now().toString(36)
    const sessionDir = join(sessionBase, sessionId)
    mkdirSync(sessionDir, { recursive: true })
    if (opts.seedMessages && opts.seedMessages.length > 0) {
      const jsonl = opts.seedMessages.map((m) => JSON.stringify(m)).join('\n') + '\n'
      writeFileSync(join(sessionDir, 'messages-current.jsonl'), jsonl)
    }
    if (opts.seedPins && Object.keys(opts.seedPins).length > 0) {
      const store = { channels: opts.seedPins }
      writeFileSync(join(sessionDir, 'pins.json'), JSON.stringify(store))
    }
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
  await waitForHealthz(backendPort)
  return { app, sessionBase, userDataDir, backendPort }
}

const waitForHealthz = async (port: number): Promise<void> => {
  const deadline = Date.now() + 15_000
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/healthz`)
      if (res.ok) return
    } catch {
      // backend not yet listening — retry
    }
    await new Promise((resolve) => setTimeout(resolve, 100))
  }
  throw new Error(`backend on port ${port} did not become healthy within 15s`)
}

export const launchIsolatedWithWindow = async (
  opts: LaunchOpts = {}
): Promise<LaunchResult & { win: Page }> => {
  const result = await launchIsolatedElectron(opts)
  const win = await result.app.firstWindow()
  return { ...result, win }
}
