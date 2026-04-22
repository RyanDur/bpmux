import { app, shell, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'path'
import { homedir } from 'os'
import { readFileSync } from 'fs'
import { spawn, ChildProcess } from 'child_process'
import * as http from 'http'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'

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
const READY_TIMEOUT_MS = 10_000
const SIGKILL_FALLBACK_MS = 2_000

let backendProcess: ChildProcess | null = null
let authToken = ''
let shuttingDown = false

function resolveBinaryPath(): string {
  if (app.isPackaged) {
    // NOTE: packaged layout is not wired for phase-1a. We'll drop the binary
    // in process.resourcesPath/bpmux when electron-builder integration lands.
    return join(process.resourcesPath, 'bpmux')
  }
  // __dirname during dev = <repo>/applications/electron/out/main.
  // Workspace root is four levels up; Cargo puts the binary in target/debug.
  return join(__dirname, '../../../../target/debug/bpmux')
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
    if (!shuttingDown) {
      // If the backend dies unexpectedly while the app is running, tear down.
      app.quit()
    }
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

function createWindow(): void {
  const mainWindow = new BrowserWindow({
    width: 900,
    height: 670,
    show: false,
    autoHideMenuBar: true,
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
