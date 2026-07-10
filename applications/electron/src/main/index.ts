import {
  app,
  shell,
  BrowserWindow,
  ipcMain,
  dialog,
  Menu,
  type MenuItemConstructorOptions
} from 'electron'
import { join } from 'path'
import { homedir } from 'os'
import { copyFileSync, existsSync, mkdirSync, readFileSync, statSync } from 'fs'
import { spawn, ChildProcess } from 'child_process'
import { randomBytes } from 'crypto'
import * as http from 'http'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import { applySelfUpdateRestartPolicy } from './selfUpdateRestartPolicy'

const BACKEND_HOST = '127.0.0.1'
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
const READY_TIMEOUT_MS = 30_000
const SIGKILL_FALLBACK_MS = 2_000

const RESTART_EXIT_CODE = 200
const RESTART_LOOP_GUARD_WINDOW_MS = 60_000
const RESTART_LOOP_GUARD_MAX = 5
const RESTART_LOOP_HALT_EXIT_CODE = 99
const RESTART_RESPAWN_DELAY_MS = 250

let backendProcess: ChildProcess | null = null
let authToken = ''
const ownerNonce = randomBytes(32).toString('hex')
let shuttingDown = false
const recentRestartsAt: number[] = []

const resolveBinaryPath = (): string => {
  if (app.isPackaged) {
    return join(process.resourcesPath, 'bpmux')
  }
  const workspace = join(__dirname, '../../../..')
  const releasePath = join(workspace, 'target/release/bpmux')
  const debugPath = join(workspace, 'target/debug/bpmux')
  const releaseExists = existsSync(releasePath)
  const debugExists = existsSync(debugPath)
  if (releaseExists && debugExists) {
    return statSync(releasePath).mtimeMs >= statSync(debugPath).mtimeMs ? releasePath : debugPath
  }
  if (releaseExists) return releasePath
  return debugPath
}

const resolveAuthTokenPath = (): string => {
  const override = process.env['BPMUX_AUTH_TOKEN_PATH']
  if (override && override.length > 0) {
    return override
  }
  return join(homedir(), '.bpmux', 'auth_token')
}

const resolveInstalledBinaryPath = (): string => {
  const override = process.env['BPMUX_BINARY_WATCH_PATH']
  if (override && override.length > 0) {
    return override
  }
  return join(homedir(), '.bpmux', 'installed', 'bpmux')
}

const stageBinaryAtInstalledPath = (sourceBinary: string): string => {
  const installed = resolveInstalledBinaryPath()
  const installedDir = join(installed, '..')
  if (!existsSync(installedDir)) {
    mkdirSync(installedDir, { recursive: true })
  }
  if (!existsSync(installed) || statSync(installed).mtimeMs < statSync(sourceBinary).mtimeMs) {
    copyFileSync(sourceBinary, installed)
  }
  return installed
}

const checkBackendReady = (): Promise<boolean> => {
  return new Promise((resolve) => {
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

const checkPortInUse = (): Promise<boolean> => {
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

const waitForBackendReady = async (): Promise<void> => {
  const start = Date.now()
  while (Date.now() - start < READY_TIMEOUT_MS) {
    if (await checkBackendReady()) return
    await new Promise((r) => setTimeout(r, READY_POLL_INTERVAL_MS))
  }
  throw new Error(
    `bpmux backend did not become ready within ${READY_TIMEOUT_MS / 1000}s on ${API_BASE}`
  )
}

const readAuthToken = (): string => {
  const tokenPath = resolveAuthTokenPath()
  const raw = readFileSync(tokenPath, 'utf8')
  return raw.trim()
}

const claimOwnerSession = (): Promise<void> => {
  return new Promise((resolve) => {
    const body = JSON.stringify({ nonce: ownerNonce })
    const req = http.request(
      {
        host: BACKEND_HOST,
        port: BACKEND_PORT,
        path: '/api/owner-session',
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(body),
          Authorization: `Bearer ${authToken}`
        }
      },
      (res) => {
        res.resume()
        if (res.statusCode !== 201) {
          console.error(`[bpmux] owner-session claim returned ${res.statusCode}`)
        }
        resolve()
      }
    )
    req.on('error', (err) => {
      console.error('[bpmux] owner-session claim failed:', err)
      resolve()
    })
    req.write(body)
    req.end()
  })
}

const spawnBackend = async (): Promise<void> => {
  console.log(`[bpmux] checking port ${BACKEND_PORT}...`)
  if (await checkPortInUse()) {
    console.log(`[bpmux] backend already running on ${BACKEND_PORT} — adopting`)
    authToken = readAuthToken()
    await claimOwnerSession()
    return
  }

  const sourceBinary = resolveBinaryPath()
  const installedBinary = stageBinaryAtInstalledPath(sourceBinary)
  const args = ['--name', 'dev-electron', '--mcp-port', String(BACKEND_PORT), 'server']

  console.log(`[bpmux] spawning ${installedBinary} ${args.join(' ')}`)
  const child = spawn(installedBinary, args, {
    stdio: ['ignore', 'inherit', 'inherit'],
    windowsHide: true,
    env: {
      ...process.env,
      BPMUX_BINARY_WATCH_PATH: installedBinary
    }
  })

  backendProcess = child

  child.on('error', (err) => {
    console.error('[bpmux] backend spawn error:', err)
    if (!shuttingDown) {
      dialog.showErrorBox(
        'bpmux backend failed to start',
        `Could not spawn bpmux binary at:\n${installedBinary}\n\n${err.message}`
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
  await claimOwnerSession()
  console.log(`[bpmux] backend ready on ${API_BASE}`)
}

const handleSelfUpdateRestart = (): void => {
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

const killBackend = (): void => {
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

const createWindow = (): void => {
  const isolateFromHostInput = process.env.BPMUX_E2E === '1'
  const mainWindow = new BrowserWindow({
    width: 900,
    height: 670,
    show: false,
    focusable: !isolateFromHostInput,
    autoHideMenuBar: true,
    titleBarStyle: 'hidden',
    titleBarOverlay: {
      height: 32
    },
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    if (isolateFromHostInput) {
      mainWindow.showInactive()
    } else {
      mainWindow.show()
    }
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  mainWindow.webContents.on('context-menu', (event, params) => {
    if (!params.selectionText) return
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

  const configPayload = (): {
    wsUrl: string
    authToken: string
    ownerNonce: string
    apiBase: string
    e2e: boolean
  } => ({
    wsUrl: WS_URL,
    authToken,
    ownerNonce,
    apiBase: API_BASE,
    e2e: process.env.BPMUX_E2E === '1'
  })

  ipcMain.handle('bpmux:config', () => configPayload())
  ipcMain.on('bpmux:config-sync', (event) => {
    event.returnValue = configPayload()
  })

  ipcMain.on('ping', () => console.log('pong'))

  try {
    await spawnBackend()
  } catch {
    return
  }

  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('will-quit', () => {
  killBackend()
})

process.on('exit', () => {
  killBackend()
})

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
