import { _electron as electron, type ElectronApplication, type Page } from '@playwright/test'
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import * as net from 'net'

// #394 shared desk-check / real-backend launch helper.
//
// Background: Ryan's live bpmux runs on port 4117. If a Playwright
// spec launches Electron without pinning a unique BPMUX_BACKEND_PORT,
// Electron's main process either (a) binds 4117 and fails with "port
// in use," or worse (b) notices 4117 is taken and "adopts" Ryan's
// live backend — so the test's fixture data writes into his real
// session log and its messages appear in his chat.
//
// This helper is the ONLY sanctioned way for a desk-check spec to
// call `electron.launch`. It:
//   - allocates a fresh ephemeral port via `getFreePort`
//   - creates a unique `BPMUX_SESSION_BASE` temp dir
//   - creates a unique Electron `--user-data-dir` so renderer
//     localStorage / cookies / indexedDB start empty
//   - optionally seeds a `messages-current.jsonl` under the session
//     base so the spawned bpmux serves fixed content from
//     `/api/messages` with no mocking
//
// Direct `electron.launch` in a session-spawning spec is prohibited;
// use this helper (#394 ACs + feedback_no_broad_pkill_during_shared_session).

/**
 * Acquire a random ephemeral port by binding + immediately releasing.
 * Non-deterministic vs a fixed port but collision-free against Ryan's
 * live backend and against parallel test processes.
 */
export async function getFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
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
}

export interface LaunchOpts {
  /**
   * Reuse an existing userDataDir across a launch pair (for tests
   * that verify the second launch reads the first launch's writes).
   * Default: fresh temp dir per launch.
   */
  userDataDir?: string
  /**
   * Reuse an existing sessionBase. Default: fresh temp dir per launch.
   */
  sessionBase?: string
  /**
   * Pin a specific backend port (e.g., when a test process needs to
   * POST back to the spawned backend). Default: fresh free port.
   */
  backendPort?: number
  /**
   * Seed `messages-current.jsonl` so the spawned bpmux serves these
   * messages through `/api/messages` without network mocking.
   */
  seedMessages?: ReadonlyArray<Record<string, unknown>>
  /**
   * Extra env vars layered on top of the isolation env. Rarely needed;
   * overrides BPMUX_BACKEND_PORT / BPMUX_SESSION_BASE at your own risk.
   */
  extraEnv?: Record<string, string>
}

export interface LaunchResult {
  app: ElectronApplication
  sessionBase: string
  userDataDir: string
  backendPort: number
}

/**
 * Launch Electron with isolated session + port + user-data-dir. Every
 * desk-check spec should use this; direct `electron.launch` bypasses
 * the isolation guarantees and risks polluting Ryan's live session.
 *
 * Caller is responsible for `await app.close()` at test end (or via
 * test.afterEach fixture).
 */
export async function launchIsolatedElectron(opts: LaunchOpts = {}): Promise<LaunchResult> {
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

/**
 * Convenience wrapper: launch + return the first window alongside the
 * isolation metadata. Most desk-check specs want this shape.
 */
export async function launchIsolatedWithWindow(
  opts: LaunchOpts = {}
): Promise<LaunchResult & { win: Page }> {
  const result = await launchIsolatedElectron(opts)
  const win = await result.app.firstWindow()
  return { ...result, win }
}
