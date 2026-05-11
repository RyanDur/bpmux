import { app, shell, BrowserWindow, ipcMain, dialog, Menu, type MenuItemConstructorOptions } from 'electron'
import { join } from 'path'
import { homedir } from 'os'
import { existsSync, readFileSync, statSync } from 'fs'
import { spawn, ChildProcess } from 'child_process'
import * as http from 'http'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'
import { applySelfUpdateRestartPolicy } from './selfUpdateRestartPolicy'

const BACKEND_HOST = '127.0.0.1'
// BACKEND_PORT is overridable via env for test isolation — Playwright
// real-backend tests can pin a per-test port so they don't adopt
// whatever bpmux is already running on 4117 (e.g., the dev's live
// session). Production + default dev run on 4117.
const BACKEND_PORT = (() => {
  const raw = process.env['BPMUX_BACKEND_PORT']
  if (!raw) return 4117
  const n = Number.parseInt(raw, 10)
  if (!Number.isFinite(n) || n <= 0 || n > 65535) return 4117
  return n
})()
const API_BASE = `http://${BACKEND_HOST}:${BACKEND_PORT}`
const WS_URL = `ws://${BACKEND_HOST}:${BACKEND_PORT}/ws`
const READY_POLL_INTERVAL_MS = 100
// 30 s readiness budget. The bpmux server's HTTP listener binds
// after `build_prod_orchestrator` completes its ingest-with-retries
// (0/500/1500/4000 ms backoffs ≈ 6 s minimum), and the initial GH
// ingest of a 500+ story canonical store can add several seconds
// of paginated fetch on top. The previous 10 s budget left no
// headroom and tripped on cold starts.
const READY_TIMEOUT_MS = 30_000
const SIGKILL_FALLBACK_MS = 2_000

const RESTART_EXIT_CODE = 200
const RESTART_LOOP_GUARD_WINDOW_MS = 60_000
const RESTART_LOOP_GUARD_MAX = 5
const RESTART_LOOP_HALT_EXIT_CODE = 99
const RESTART_RESPAWN_DELAY_MS = 250

let backendProcess: ChildProcess | null = null
let authToken = ''
let shuttingDown = false
const recentRestartsAt: number[] = []

function resolveBinaryPath(): string {
  if (app.isPackaged) {
    // NOTE: packaged layout is not wired for phase-1a. We'll drop the binary
    // in process.resourcesPath/bpmux when electron-builder integration lands.
    return join(process.resourcesPath, 'bpmux')
  }
  // __dirname during dev = <repo>/applications/electron/out/main.
  // Workspace root is four levels up. Prefer the freshest of
  // target/release vs target/debug — startup latency on a 500+ story
  // canonical-store ingest can blow the 10 s electron readiness
  // timeout on a debug binary, so when both are present we pick the
  // newer one. `cargo build [--release]` updates the relevant
  // mtime; whichever build the developer just ran wins.
  const workspace = join(__dirname, '../../../..')
  const releasePath = join(workspace, 'target/release/bpmux')
  const debugPath = join(workspace, 'target/debug/bpmux')
  const releaseExists = existsSync(releasePath)
  const debugExists = existsSync(debugPath)
  if (releaseExists && debugExists) {
    return statSync(releasePath).mtimeMs >= statSync(debugPath).mtimeMs
      ? releasePath
      : debugPath
  }
  if (releaseExists) return releasePath
  return debugPath
}

function resolveAuthTokenPath(): string {
  const override = process.env['BPMUX_AUTH_TOKEN_PATH']
  if (override && override.length > 0) {
    return override
  }
  return join(homedir(), '.bpmux', 'auth_token')
}

function checkBackendReady(): Promise<boolean> {
  return new Promise((resolve) => {
    // Use the dedicated /healthz liveness route (unauth, constant-size)
    // instead of /api/messages, which returns the full session log and
    // gets slower than the readiness timeout once the log grows past
    // a few hundred KB. See `applications/bpmux/src/server.rs`.
    const req = http.request(
      {
        host: BACKEND_HOST,
        port: BACKEND_PORT,
        path: '/healthz',
        method: 'GET',
        timeout: READY_POLL_INTERVAL_MS
      },
      (res) => {
        res.resume()
        resolve(res.statusCode === 200)
      }
    )
    req.on('error', () => resolve(false))
    req.on('timeout', () => {
      req.destroy()
      resolve(false)
    })
    req.end()
  })
}

function checkPortInUse(): Promise<boolean> {
  // If something is already listening on 4117, we'd see a successful connect.
  return new Promise((resolve) => {
    const req = http.request(
      {
        host: BACKEND_HOST,
        port: BACKEND_PORT,
        path: '/api/messages',
        method: 'GET',
        timeout: 500
      },
      (res) => {
        res.resume()
        resolve(true)
      }
    )
    req.on('error', () => resolve(false))
    req.on('timeout', () => {
      req.destroy()
      resolve(false)
    })
    req.end()
  })
}

async function waitForBackendReady(): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < READY_TIMEOUT_MS) {
    if (await checkBackendReady()) return
    await new Promise((r) => setTimeout(r, READY_POLL_INTERVAL_MS))
  }
  throw new Error(
    `bpmux backend did not become ready within ${READY_TIMEOUT_MS / 1000}s on ${API_BASE}`
  )
}

function readAuthToken(): string {
  const tokenPath = resolveAuthTokenPath()
  const raw = readFileSync(tokenPath, 'utf8')
  return raw.trim()
}

async function spawnBackend(): Promise<void> {
  console.log(`[bpmux] checking port ${BACKEND_PORT}...`)
  if (await checkPortInUse()) {
    console.log(`[bpmux] backend already running on ${BACKEND_PORT} — adopting`)
    authToken = readAuthToken()
    return
  }

  const binary = resolveBinaryPath()
  const args = ['--name', 'dev-electron', '--mcp-port', String(BACKEND_PORT), 'server']

  console.log(`[bpmux] spawning ${binary} ${args.join(' ')}`)
  const child = spawn(binary, args, {
    stdio: ['ignore', 'inherit', 'inherit'],
    windowsHide: true
  })

  backendProcess = child

  child.on('error', (err) => {
    console.error('[bpmux] backend spawn error:', err)
    if (!shuttingDown) {
      dialog.showErrorBox(
        'bpmux backend failed to start',
        `Could not spawn bpmux binary at:\n${binary}\n\n${err.message}`
      )
      app.exit(1)
    }
  })

  child.on('exit', (code, signal) => {
    console.log(`[bpmux] backend exited code=${code} signal=${signal}`)
    backendProcess = null
    if (shuttingDown) return
    if (code === RESTART_EXIT_CODE) {
      handleSelfUpdateRestart()
      return
    }
    app.quit()
  })

  try {
    await waitForBackendReady()
  } catch (err) {
    dialog.showErrorBox('bpmux backend not ready', err instanceof Error ? err.message : String(err))
    killBackend()
    app.exit(1)
    throw err
  }

  authToken = readAuthToken()
  console.log(`[bpmux] backend ready on ${API_BASE}`)
}

function handleSelfUpdateRestart(): void {
  const decision = applySelfUpdateRestartPolicy(recentRestartsAt, Date.now(), {
    windowMs: RESTART_LOOP_GUARD_WINDOW_MS,
    maxRestarts: RESTART_LOOP_GUARD_MAX
  })
  if (decision.action === 'halt') {
    console.error(
      `[bpmux] self-update halt: ${decision.restartsInWindow} restarts in ${RESTART_LOOP_GUARD_WINDOW_MS / 1000}s — likely bad-binary thrash`
    )
    dialog.showErrorBox(
      'bpmux self-update halted',
      `Backend restart-loop guard tripped (${decision.restartsInWindow} restarts in ${RESTART_LOOP_GUARD_WINDOW_MS / 1000}s). Investigate the binary before relaunching.`
    )
    app.exit(RESTART_LOOP_HALT_EXIT_CODE)
    return
  }
  console.log(
    `[bpmux] self-update: respawning backend (restart #${decision.restartsInWindow} in window)`
  )
  setTimeout(() => {
    spawnBackend().catch((err) => {
      console.error('[bpmux] self-update respawn failed:', err)
      if (!shuttingDown) app.quit()
    })
  }, RESTART_RESPAWN_DELAY_MS)
}


function killBackend(): void {
  const child = backendProcess
  if (!child || child.killed) return
  shuttingDown = true
  try {
    child.kill('SIGTERM')
  } catch (err) {
    console.error('[bpmux] SIGTERM failed:', err)
  }
  setTimeout(() => {
    if (child && !child.killed && backendProcess === child) {
      try {
        child.kill('SIGKILL')
      } catch (err) {
        console.error('[bpmux] SIGKILL failed:', err)
      }
    }
  }, SIGKILL_FALLBACK_MS)
}

// #1286 Option 2: off-screen window-positioning for Playwright e2e
// tests on bpmux-mac-m1 self-hosted runner. When tests run on Ryan's
// dev machine, visible test-spawned windows visually disrupt his
// live work. Option 1 (hidden-windows via show:false sole-control)
// was retired because macOS throttles hidden windows below 10s
// timeouts (commit f9ebe47), breaking real-backend + human-observes
// tests. Off-screen positioning keeps windows technically active
// (no throttle) but outside any reasonable monitor's coordinate
// space. Coordinate (-10000, -10000) is well below any practical
// multi-monitor setup's leftmost extent.
const OFFSCREEN_X = -10000
const OFFSCREEN_Y = -10000

function shouldPositionOffscreen(): boolean {
  return process.env['BPMUX_TEST_WINDOW_POSITION'] === 'offscreen'
}

function createWindow(): void {
  const offscreen = shouldPositionOffscreen()
  const mainWindow = new BrowserWindow({
    width: 900,
    height: 670,
    show: false,
    autoHideMenuBar: true,
    ...(offscreen ? { x: OFFSCREEN_X, y: OFFSCREEN_Y } : {}),
    ...(process.platform === 'linux' ? { icon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  // #371: right-click context menu integration for the quote-
  // excerpt feature. Builds a native Menu so Copy / Paste / Inspect
  // Element remain available alongside the custom "Quote in reply"
  // item (the Cmd+E keyboard path shipped in #366). Main builds the
  // menu because only main has access to Electron's Menu API; the
  // renderer signals quote-readiness by setting a data attribute on
  // the selected comment's <li>, which main reads via a short
  // executeJavaScript probe at context-menu time.
  mainWindow.webContents.on('context-menu', (event, params) => {
    if (!params.selectionText) return
    // Probe: is the selection inside a comment? If so, return its
    // id + author so the click handler has the metadata it needs to
    // dispatch the quote to the renderer.
    mainWindow.webContents
      .executeJavaScript(
        `(() => {
           const sel = window.getSelection();
           if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return null;
           let node = sel.anchorNode;
           while (node && !(node instanceof HTMLElement && node.dataset.commentId)) {
             node = node.parentNode;
           }
           if (!(node instanceof HTMLElement)) return null;
           return {
             commentId: node.dataset.commentId,
             author: node.dataset.commentAuthor || 'unknown',
             text: sel.toString()
           };
         })()`
      )
      .then((ctx: { commentId: string; author: string; text: string } | null) => {
        const template: MenuItemConstructorOptions[] = [
          { role: 'copy' },
          { role: 'paste' },
          { type: 'separator' },
          { role: 'selectAll' }
        ]
        if (ctx && ctx.commentId) {
          // Prepend the quote item so it's visually prominent (above
          // the standard clipboard items). Matches macOS context-menu
          // conventions where contextual-first actions lead.
          template.unshift(
            {
              label: 'Quote in reply',
              click: () => {
                mainWindow.webContents.send('bpmux:quote-comment', ctx)
              }
            },
            { type: 'separator' }
          )
        }
        if (is.dev) {
          template.push({ type: 'separator' }, { role: 'toggleDevTools' })
        }
        const menu = Menu.buildFromTemplate(template)
        menu.popup({ window: mainWindow, x: params.x, y: params.y })
      })
      .catch((err: unknown) => {
        console.error('[bpmux] context-menu probe failed:', err)
      })
    event.preventDefault()
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

app.whenReady().then(async () => {
  electronApp.setAppUserModelId('com.electron')

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  const configPayload = (): { wsUrl: string; authToken: string; apiBase: string } => ({
    wsUrl: WS_URL,
    authToken,
    apiBase: API_BASE
  })

  // Async form (per spec) + sync form (preload uses sendSync so the exposed
  // fields are populated before the renderer evaluates; both return the same
  // payload).
  ipcMain.handle('bpmux:config', () => configPayload())
  ipcMain.on('bpmux:config-sync', (event) => {
    event.returnValue = configPayload()
  })

  ipcMain.on('ping', () => console.log('pong'))

  try {
    await spawnBackend()
  } catch {
    // spawnBackend already surfaced a dialog + triggered app.exit.
    return
  }

  createWindow()

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('will-quit', () => {
  killBackend()
})

process.on('exit', () => {
  killBackend()
})

// Forward common termination signals so the child doesn't orphan.
for (const sig of ['SIGINT', 'SIGTERM', 'SIGHUP'] as const) {
  process.on(sig, () => {
    killBackend()
    app.quit()
  })
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})
